#!/bin/sh
SD="${SLOT_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
RUN="${AGS_RUN:-/run}"
LOGS="$SD/Logs"
SESSION_MAX=65536
export SLOT_ROOT="$SD"

mkdir -p "$LOGS" 2>/dev/null
for f in slot.log slot.log.1 slot-session.log slot-session.log.1 slot-card.log ags-shutdown-trace.log input-trace.log; do
	[ -f "$SD/$f" ] && mv -f "$SD/$f" "$LOGS/$f"
done
[ -d "$SD/crash" ] && [ ! -e "$LOGS/crash" ] && mv "$SD/crash" "$LOGS/crash"

[ $(wc -c < "$LOGS/slot-session.log" 2>/dev/null || echo 0) -gt "$SESSION_MAX" ] \
	&& mv -f "$LOGS/slot-session.log" "$LOGS/slot-session.log.1"

log() {
	echo "$(date '+%F %T') $*" >> "$LOGS/slot-session.log" 2>/dev/null
}

pid="$(cat "$RUN/slot-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/led.sh" once
	sh "$SYS/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-led.pid"
fi

pid="$(cat "$RUN/slot-lid.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/lid.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-lid.pid"
fi

echo "|/bin/sh $SYS/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

for governor in /sys/devices/system/cpu/cpufreq/policy*/scaling_governor; do
	[ -w "$governor" ] || continue
	printf '%s\n' schedutil > "$governor" 2>/dev/null || log "could not set schedutil for $governor"
done

for usb in \
	/sys/devices/platform/soc/5200000.ohci1-controller/ohci_enable \
	/sys/devices/platform/soc/5310000.ohci2-controller/ohci_enable \
	/sys/devices/platform/soc/5311000.ohci3-controller/ohci_enable \
	/sys/devices/platform/soc/5200000.ehci1-controller/ehci_enable \
	/sys/devices/platform/soc/5310000.ehci2-controller/ehci_enable \
	/sys/devices/platform/soc/5311000.ehci3-controller/ehci_enable; do
	[ -w "$usb" ] && printf '0\n' > "$usb" 2>/dev/null
done

[ -f "$RUN/slotlink.session" ] && sh "$SYS/slotlink.sh" link down > /dev/null 2>&1

if [ -d /sys/devices/platform/soc ]; then
	(
		sleep 20
		[ -f "$RUN/slotlink.session" ] || rmmod 8821cs 2>/dev/null || true
	) > /dev/null 2>&1 &
fi

[ -f "$LOGS/slot.log" ] && mv -f "$LOGS/slot.log" "$LOGS/slot.log.1"
log "exec $SYS/slot"

exec /lib/ld-linux-aarch64.so.1 "$SYS/slot" > "$LOGS/slot.log" 2>&1
