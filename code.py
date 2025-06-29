import time
import board
import analogio
import array
import adafruit_wave
import supervisor
import busio
import displayio
import framebufferio
import sharpdisplay

# --- Setup Display ---
displayio.release_displays()
disp_dat = board.GP19
disp_sck = board.GP18
disp_cs = board.GP17
spi_bus = busio.SPI(clock=disp_sck, MOSI=disp_dat)
framebuffer = sharpdisplay.SharpMemoryFramebuffer(spi_bus, disp_cs, 400, 240)
display = framebufferio.FramebufferDisplay(framebuffer, auto_refresh=False)
display.rotation = 180

# Setup ADC
adc = analogio.AnalogIn(board.A0)

# WAV file parameters
SAMPLE_RATE = 8000
DURATION_SEC = 3
NUM_SAMPLES = SAMPLE_RATE * DURATION_SEC

# Setup array for samples
samples = array.array("H", [0] * NUM_SAMPLES)

# Record audio
print("Recording...")
for i in range(NUM_SAMPLES):
    samples[i] = adc.value  # 16-bit value from 0 to 65535
    time.sleep(1 / SAMPLE_RATE)
print("Done recording.")

# Save as WAV file
filename = "/audio.wav"
with open(filename, "wb") as f:
    wav = adafruit_wave.open(f, "wb")
    wav.setnchannels(1)
    wav.setsampwidth(2)
    wav.setframerate(SAMPLE_RATE)
    wav.writeframes(samples.tobytes())
    wav.close()

print("Saved to", filename)

# Stop here so host can read file over USB
supervisor.reload()


"""
import audiocore, audiomixer, audiobusio
import time
import board
import busio
import digitalio
import displayio
import framebufferio
import sharpdisplay
from digitalio import Direction, DriveMode
from adafruit_display_text.label import Label
from terminalio import FONT
import adafruit_gps

 
root_group = displayio.Group()
display.root_group = root_group

labels = {}
fields = ["Time", "Latitude", "Longitude", "Altitude", "Heading", "Grid Square", "Sats in View", "GPS Fix"]
for i, name in enumerate(fields):
    label = Label(font=FONT, text=f"{name}: -", x=5, y=15 + i * 20, scale=2)
    labels[name] = label
    root_group.append(label)
 
# --- Setup PTT ---
ptt = digitalio.DigitalInOut(board.GP15)
ptt.direction = Direction.OUTPUT
ptt.drive_mode = DriveMode.OPEN_DRAIN
ptt.value = True
def assert_ptt(key: bool):
    ptt.value = not key

# --- Setup GPS on UART0 ---
uart = busio.UART(board.GP0, board.GP1, baudrate=38400, timeout=1)
gps = adafruit_gps.GPS(uart, debug=False)
gps.send_command(b'PMTK220,1000')  # Update rate: 1Hz

# RMC, VTG, GGA
# RecMin, Course/Ground Speed, FixData, DOPS
gps.send_command(b'PMTK314,0,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0')  

def latlon_to_grid(lat, lon):
    lat += 90
    lon += 180

    A = ord('A')
    a = ord('a')
    grid = ""

    # First pair: field (20° lon × 10° lat)
    grid += chr(A + int(lon // 20))
    grid += chr(A + int(lat // 10))

    # Second pair: square (2° lon × 1° lat)
    grid += str(int((lon % 20) // 2))
    grid += str(int(lat % 10))

    # Third pair: subsquare (5′ lon × 2.5′ lat)
    grid += chr(a + int((lon % 2) * 12))
    grid += chr(a + int((lat % 1) * 24))

    return grid


# --- Main loop ---
last_print = time.monotonic()
while True:
    gps.update()
    now = time.monotonic()

    if now - last_print <= 0.5:
        continue

    last_print = now

    lat = gps.latitude
    lon = gps.longitude

    if gps.has_fix and gps.timestamp_utc:
        t = gps.timestamp_utc
        labels["Time"].text = f"Time: {t.tm_hour:02}:{t.tm_min:02}:{t.tm_sec:02}"
    else:
        labels["Time"].text = "Time: -"

    labels["Latitude"].text  = f"Latitude: {gps.latitude:.6f}" if gps.has_fix else "Latitude: -"
    labels["Longitude"].text = f"Longitude: {gps.longitude:.6f}" if gps.has_fix else "Longitude: -"
    labels["Altitude"].text  = f"Altitude: {gps.altitude_m:.1f} m" if gps.has_fix and gps.altitude_m is not None else "Altitude: -"
    labels["Heading"].text   = f"Heading: {gps.track_angle_deg:.1f}°" if gps.has_fix and gps.track_angle_deg is not None else "Heading: -"
    labels["Grid Square"].text = f"Grid Square: {latlon_to_grid(lat, lon)}" if gps.has_fix else "Grid Square: -"
    labels["Sats in View"].text = f"Sats in View: {gps.satellites}" if gps.has_fix else "Sats in View: -"
    labels["GPS Fix"].text   = f"GPS Fix: {'Yes' if gps.has_fix else 'No'}"

    display.refresh()



# --- Setup audio output ---
i2s_bclk = board.GP21 # BCK on PCM5102
i2s_wsel = board.GP22 # LCK on PCM5102
i2s_data = board.GP20 # DIN on PCM5102
audio = audiobusio. I2SOut(bit_clock=i2s_bclk, word_select=i2s_wsel, data=i2s_data)
mixer = audiomixer.Mixer(voice_count=1, sample_rate=44100, channel_count=1,
                         bits_per_sample=16, samples_signed=True)
audio.play(mixer) # attach mixer to audio playback
 
fname = "/aprs.wav"
 
while True:
    assert_ptt(True)
    wave = audiocore.WaveFile(open(fname, "rb"))
    mixer.voice[0].play(wave, loop=False )
    while mixer.voice[0].playing:
        pass
    mixer.voice[0].stop()
    assert_ptt(False)
    time.sleep(3)

"""