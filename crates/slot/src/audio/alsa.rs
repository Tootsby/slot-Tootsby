use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use libloading::Library;

use super::pacer::Pacer;
use super::ring::Ring;
use super::silence::Silence;
use super::sink::{AudioError, AudioSink};

const SND_PCM_STREAM_PLAYBACK: c_int = 0;
const SND_PCM_FORMAT_S16_LE: c_int = 2;
const SND_PCM_ACCESS_RW_INTERLEAVED: c_int = 3;
const CHANNELS: c_uint = 2;

const LATENCY_US: c_uint = 40_000;

const PERIOD_FRAMES: usize = 512;

const RELEASE_AFTER: Duration = Duration::from_secs(3);

const DEVICES: [&str; 4] = ["plug:default", "default", "plughw:0,0", "hw:0,0"];

type PcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type PcmSetParams =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type PcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> i64;
type PcmRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type PcmDrop = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcmClose = unsafe extern "C" fn(*mut c_void) -> c_int;
type StrError = unsafe extern "C" fn(c_int) -> *const c_char;

struct Alsa {
    open: PcmOpen,
    set_params: PcmSetParams,
    writei: PcmWritei,
    recover: PcmRecover,
    drop: PcmDrop,
    close: PcmClose,
    strerror: StrError,
    _lib: Library,
}

impl Alsa {
    fn load() -> Result<Self, AudioError> {
        let lib = unsafe { Library::new("libasound.so.2") }
            .or_else(|_| unsafe { Library::new("libasound.so") })
            .map_err(|e| AudioError::Device(format!("libasound: {e}")))?;
        macro_rules! get {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())
                    .map_err(|e| AudioError::Device(format!("{}: {e}", $name)))?
            };
        }
        unsafe {
            Ok(Alsa {
                open: get!("snd_pcm_open"),
                set_params: get!("snd_pcm_set_params"),
                writei: get!("snd_pcm_writei"),
                recover: get!("snd_pcm_recover"),
                drop: get!("snd_pcm_drop"),
                close: get!("snd_pcm_close"),
                strerror: get!("snd_strerror"),
                _lib: lib,
            })
        }
    }

    fn message(&self, err: c_int) -> String {
        let text = unsafe { (self.strerror)(err) };
        match text.is_null() {
            true => format!("error {err}"),
            false => unsafe { CStr::from_ptr(text) }
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn open_pcm(&self, rate: u32) -> Result<*mut c_void, AudioError> {
        let mut last = AudioError::NoDevice;
        for name in DEVICES {
            let Ok(cname) = CString::new(name) else {
                continue;
            };
            let mut pcm: *mut c_void = std::ptr::null_mut();
            let err = unsafe { (self.open)(&mut pcm, cname.as_ptr(), SND_PCM_STREAM_PLAYBACK, 0) };
            if err < 0 || pcm.is_null() {
                last = AudioError::Device(format!("{name}: {}", self.message(err)));
                continue;
            }
            let err = unsafe {
                (self.set_params)(
                    pcm,
                    SND_PCM_FORMAT_S16_LE,
                    SND_PCM_ACCESS_RW_INTERLEAVED,
                    CHANNELS,
                    rate,
                    1,
                    LATENCY_US,
                )
            };
            if err < 0 {
                unsafe { (self.close)(pcm) };
                last = AudioError::Config(format!("{name}: {}", self.message(err)));
                continue;
            }
            eprintln!("slot: audio {name} at {rate} Hz");
            return Ok(pcm);
        }
        Err(last)
    }
}

pub struct AlsaSink {
    ring: Arc<Ring>,
    device: Option<Device>,
}

struct Device {
    stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

impl AlsaSink {
    pub fn new() -> Self {
        AlsaSink {
            ring: Arc::new(Ring::new(0)),
            device: None,
        }
    }

    fn close(&mut self) {
        if let Some(d) = self.device.take() {
            d.stop.store(true, Ordering::Relaxed);
            let _ = d.join.join();
        }
    }
}

impl Default for AlsaSink {
    fn default() -> Self {
        AlsaSink::new()
    }
}

impl Drop for AlsaSink {
    fn drop(&mut self) {
        self.close();
    }
}

impl AudioSink for AlsaSink {
    fn open(&mut self, sample_rate: u32) -> Result<(), AudioError> {
        self.close();
        let ring = self.ring.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("slot-audio".into())
            .spawn(move || match play(&ring, sample_rate) {
                Ok(mut device) => {
                    let _ = ready_tx.send(Ok(()));
                    device.run(&ring, &flag);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| AudioError::Device(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.device = Some(Device { stop, join });
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(_) => {
                let _ = join.join();
                Err(AudioError::Device("output thread stopped".into()))
            }
        }
    }

    fn ring(&self) -> Arc<Ring> {
        self.ring.clone()
    }
}

/// The device side of the output thread: ALSA here, a stand-in in the tests below.
trait Output {
    fn open(&mut self, rate: u32) -> Result<(), AudioError>;
    fn is_open(&self) -> bool;
    /// One period, `PERIOD_FRAMES` frames. False once the device is lost for good.
    fn write(&mut self, buf: &[i16]) -> bool;
    fn release(&mut self);
}

struct AlsaOutput {
    alsa: Alsa,
    pcm: Option<*mut c_void>,
}

impl Output for AlsaOutput {
    fn open(&mut self, rate: u32) -> Result<(), AudioError> {
        self.pcm = Some(self.alsa.open_pcm(rate)?);
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.pcm.is_some()
    }

    fn write(&mut self, buf: &[i16]) -> bool {
        let Some(pcm) = self.pcm else {
            return false;
        };
        let mut written = 0;
        while written < PERIOD_FRAMES {
            let at = written * CHANNELS as usize;
            let frames = unsafe {
                (self.alsa.writei)(
                    pcm,
                    buf[at..].as_ptr() as *const c_void,
                    (PERIOD_FRAMES - written) as u64,
                )
            };
            if frames < 0 {
                let err = unsafe { (self.alsa.recover)(pcm, frames as c_int, 1) };
                if err < 0 {
                    eprintln!("slot: audio: {}", self.alsa.message(err));
                    return false;
                }
                continue;
            }
            written += frames as usize;
        }
        true
    }

    fn release(&mut self) {
        if let Some(pcm) = self.pcm.take() {
            unsafe {
                (self.alsa.drop)(pcm);
                (self.alsa.close)(pcm);
            }
        }
    }
}

impl Drop for AlsaOutput {
    fn drop(&mut self) {
        self.release();
    }
}

struct Playback<O: Output> {
    out: O,
    rate: u32,
    failing: bool,
}

fn play(ring: &Arc<Ring>, sample_rate: u32) -> Result<Playback<AlsaOutput>, AudioError> {
    let mut out = AlsaOutput {
        alsa: Alsa::load()?,
        pcm: None,
    };
    let failing = match out.open(sample_rate) {
        Ok(()) => false,
        Err(e) => {
            eprintln!("slot: audio: {e}, retrying");
            true
        }
    };
    ring.reopen(sample_rate);
    Ok(Playback {
        out,
        rate: sample_rate,
        failing,
    })
}

impl<O: Output> Playback<O> {
    fn run(&mut self, ring: &Ring, stop: &AtomicBool) {
        let mut buf = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        let span = Duration::from_secs_f64(PERIOD_FRAMES as f64 / self.rate as f64);
        let mut silence = Silence::new(RELEASE_AFTER);
        let mut pace = Pacer::new(span);
        while !stop.load(Ordering::Relaxed) {
            ring.fill(&mut buf);
            let hold = silence.hear(&buf, span);
            match (self.out.is_open(), hold) {
                (true, false) => {
                    self.out.release();
                    eprintln!("slot: audio released after silence");
                }
                (false, true) => match self.out.open(self.rate) {
                    Ok(()) => {
                        eprintln!("slot: audio reopened at {:.1} ms", crate::boot_time::ms());
                        self.failing = false;
                        if !self.prime() {
                            return;
                        }
                    }
                    Err(e) => {
                        if !self.failing {
                            eprintln!("slot: audio: {e}, retrying");
                            self.failing = true;
                        }
                    }
                },
                _ => {}
            }
            if self.out.is_open() {
                pace.reset();
                if !self.out.write(&buf) {
                    return;
                }
            } else {
                std::thread::sleep(pace.wait(Instant::now()));
            }
        }
    }

    /// A device opened after silence takes a whole buffer before it plays. Give it silence for
    /// that, the way the first open primes the ring, so it does not take that buffer from the
    /// sound queued behind this period: that sound is the cushion against a late frame.
    fn prime(&mut self) -> bool {
        let silence = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        let periods = self.rate as usize * LATENCY_US as usize / 1_000_000 / PERIOD_FRAMES;
        (0..periods).all(|_| self.out.write(&silence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::ring_capacity;

    /// What LATENCY_US asks ALSA for at 48 kHz.
    const BUFFER_FRAMES: usize = 1920;

    /// Takes periods into its buffer, as a newly opened ALSA device does, and reports full at
    /// the point ALSA would start playing, which ends the run.
    struct Buffered {
        open: bool,
        took: Vec<i16>,
    }

    impl Output for Buffered {
        fn open(&mut self, _rate: u32) -> Result<(), AudioError> {
            self.open = true;
            Ok(())
        }

        fn is_open(&self) -> bool {
            self.open
        }

        fn write(&mut self, buf: &[i16]) -> bool {
            self.took.extend_from_slice(buf);
            self.took.len() / (CHANNELS as usize) < BUFFER_FRAMES
        }

        fn release(&mut self) {
            self.open = false;
        }
    }

    #[test]
    fn a_device_reopened_after_silence_fills_up_on_silence_and_leaves_the_cushion() {
        let rate = 48_000;
        let ring = Ring::new(ring_capacity(rate));
        let cushion = ring.capacity_frames() / 2;
        let sound: Vec<i16> = (0..cushion * 2).map(|i| 1 + (i % 1000) as i16).collect();
        ring.push(&sound);
        let mut playback = Playback {
            out: Buffered {
                open: false,
                took: Vec::new(),
            },
            rate,
            failing: false,
        };
        playback.run(&ring, &AtomicBool::new(false));
        assert_eq!(
            ring.queued_frames(),
            cushion - PERIOD_FRAMES,
            "the reopened device filled its buffer from the cushion instead of from silence"
        );
        let took = &playback.out.took;
        let quiet = took.len() - PERIOD_FRAMES * CHANNELS as usize;
        assert!(
            took[..quiet].iter().all(|&s| s == 0),
            "the device was given sound before its silence"
        );
        assert_eq!(
            &took[quiet..],
            &sound[..PERIOD_FRAMES * CHANNELS as usize],
            "the sound that woke the device does not follow the silence straight on"
        );
    }
}
