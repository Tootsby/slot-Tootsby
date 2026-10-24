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

/// How long closing waits for the fade out to play before giving up on the output thread.
const CLOSE_WAIT: Duration = Duration::from_millis(500);

const DEVICES: [&str; 4] = ["plug:default", "default", "plughw:0,0", "hw:0,0"];

type PcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type PcmSetParams =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type PcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> i64;
type PcmRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type PcmDrop = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcmDrain = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcmClose = unsafe extern "C" fn(*mut c_void) -> c_int;
type StrError = unsafe extern "C" fn(c_int) -> *const c_char;

struct Alsa {
    open: PcmOpen,
    set_params: PcmSetParams,
    writei: PcmWritei,
    recover: PcmRecover,
    drop: PcmDrop,
    drain: PcmDrain,
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
                drain: get!("snd_pcm_drain"),
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
    done: mpsc::Receiver<()>,
}

impl AlsaSink {
    pub fn new() -> Self {
        AlsaSink {
            ring: Arc::new(Ring::new(0)),
            device: None,
        }
    }

    fn stop_device(&mut self) {
        if let Some(d) = self.device.take() {
            d.stop.store(true, Ordering::Relaxed);
            // A device that stops answering mid-fade must not hold up a power off: give up on
            // the thread and let the process take it down.
            if d.done.recv_timeout(CLOSE_WAIT).is_ok() {
                let _ = d.join.join();
            } else {
                eprintln!("slot: audio: the output thread did not finish its fade out");
            }
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
        self.stop_device();
    }
}

impl AudioSink for AlsaSink {
    fn open(&mut self, sample_rate: u32) -> Result<(), AudioError> {
        self.stop_device();
        let ring = self.ring.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("slot-audio".into())
            .spawn(move || {
                match play(&ring, sample_rate) {
                    Ok(mut device) => {
                        let _ = ready_tx.send(Ok(()));
                        device.run(&ring, &flag);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
                let _ = done_tx.send(());
            })
            .map_err(|e| AudioError::Device(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.device = Some(Device { stop, join, done });
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

    fn close(&mut self) {
        self.stop_device();
    }
}

struct Playback {
    alsa: Alsa,
    pcm: Option<*mut c_void>,
    rate: u32,
    failing: bool,
}

fn play(ring: &Arc<Ring>, sample_rate: u32) -> Result<Playback, AudioError> {
    let alsa = Alsa::load()?;
    ring.reopen(sample_rate);
    Ok(Playback {
        alsa,
        pcm: None,
        rate: sample_rate,
        failing: false,
    })
}

impl Playback {
    fn run(&mut self, ring: &Ring, stop: &AtomicBool) {
        let mut buf = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        let span = Duration::from_secs_f64(PERIOD_FRAMES as f64 / self.rate as f64);
        let mut silence = Silence::quiet(RELEASE_AFTER);
        let mut pace = Pacer::new(span);
        while !stop.load(Ordering::Relaxed) {
            ring.fill(&mut buf);
            let hold = silence.hear(&buf, span);
            match (self.pcm, hold) {
                (Some(_), false) => {
                    self.release();
                    eprintln!("slot: audio released after silence");
                }
                (None, true) => match self.alsa.open_pcm(self.rate) {
                    Ok(pcm) => {
                        eprintln!("slot: audio reopened at {:.1} ms", crate::boot_time::ms());
                        self.pcm = Some(pcm);
                        self.failing = false;
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
            match self.pcm {
                Some(pcm) => {
                    pace.reset();
                    if !self.write(pcm, &buf) {
                        return;
                    }
                }
                None => std::thread::sleep(pace.wait(Instant::now())),
            }
        }
        self.fade_out(ring);
    }

    /// Stopping with the device open: ramp the next period down to nothing, follow it with a
    /// buffer of silence so the output comes to rest at zero, and let all of it play before
    /// closing. Closing on live sound, or letting a power off cut it, pops the speaker.
    fn fade_out(&mut self, ring: &Ring) {
        let Some(pcm) = self.pcm else {
            return;
        };
        let mut buf = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        ring.fill(&mut buf);
        fade(&mut buf);
        let silence = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        let periods = self.rate as usize * LATENCY_US as usize / 1_000_000 / PERIOD_FRAMES;
        if self.write(pcm, &buf) && (0..periods).all(|_| self.write(pcm, &silence)) {
            self.pcm = None;
            unsafe {
                (self.alsa.drain)(pcm);
                (self.alsa.close)(pcm);
            }
        } else {
            self.release();
        }
    }

    fn write(&self, pcm: *mut c_void, buf: &[i16]) -> bool {
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

impl Drop for Playback {
    fn drop(&mut self) {
        self.release();
    }
}

/// Scales interleaved stereo frames down a straight line, from full at the first frame to
/// silence at the last.
fn fade(buf: &mut [i16]) {
    let frames = buf.len() / CHANNELS as usize;
    let last = frames.saturating_sub(1).max(1) as i32;
    for (f, frame) in buf.chunks_mut(CHANNELS as usize).enumerate() {
        let left = last - f as i32;
        for s in frame {
            *s = (i32::from(*s) * left / last) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_runs_from_full_down_to_silence_without_turning_back() {
        let mut buf = vec![i16::MAX; PERIOD_FRAMES * CHANNELS as usize];
        fade(&mut buf);
        let period = buf.len();
        assert_eq!(
            buf[..2],
            [i16::MAX, i16::MAX],
            "the fade did not start at full"
        );
        assert_eq!(buf[period - 2..], [0, 0], "the fade did not end in silence");
        assert!(
            buf.windows(2).all(|w| w[1] <= w[0]),
            "the fade went back up on its way down"
        );
        assert!(
            buf.chunks(2).all(|f| f[0] == f[1]),
            "the two channels were faded apart"
        );
    }
}
