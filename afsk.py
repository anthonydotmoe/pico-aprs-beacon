# Proof of concept APRS modulator for
# reference when building the Rust version

import numpy as np
import wave
import struct

# Parameters
SAMPLE_RATE = 8000  # Hz
BIT_RATE = 1200      # baud
LUT_SIZE = 256       # samples
# How many flags to fill 500ms of airtime?
PREPEND = int(BIT_RATE / 16)
POSTPEND = 3
# Frequencies
MARK  = 1200
SPACE = 2200

def bit_stuff(bits):
    stuffed = []
    count = 0
    for bit in bits:
        stuffed.append(bit)
        if bit == 1:
            count += 1
            if count == 5:
                stuffed.append(0) # Insert 0 after five 1s
                count = 0
        else:
            count = 0
    return stuffed

def nrzi_encode(bits, initial=0):
    encoded = []
    state = initial
    for bit in bits:
        if bit == 0:
            state ^= 1  # transition
        # else: no change
        encoded.append(state)
    return encoded

def crc16_ccitt(data):
    crc = 0xFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return crc ^ 0xFFFF

def encode_ax25_callsign(callsign:str, ssid=0, last=False):
    callsign = callsign.upper().ljust(6) # Pad to 6 chars
    result = [(ord(c) << 1) for c in callsign]

    # SSID byte:
    # Bits: 011xxxx0
    ssid_byte = 0b01100000 | ((ssid & 0x0F) << 1)
    if last:
        ssid_byte |= 0b00000001 # Set end-of-address bit
    
    result.append(ssid_byte)
    return result

def build_ax25_frame():
    frame = []

    # Destination: APRS-0
    frame += encode_ax25_callsign("APZ")

    # Source: N0CALL-7 (Handheld/portable)
    frame += encode_ax25_callsign("N0CALL", 7)

    # Digipeater path
    frame += encode_ax25_callsign("WIDE1", 1, last=True)

    frame.append(0x03)  # UI frame
    frame.append(0xF0)  # No Layer 3 protocol

    # Encode:
    # Lat/Long: 49°03'30"N 72°01'45"W
    # Symbol: Bicycle
    # Power, Effective Antenna Height/Gain/Directivity for GD-77 with HYS antenna: 1W, 0ft, 2dB, omni
    # Comment: "Test 001234"
    aprs_payload = "!4903.50N/07201.75WbPHG0020Test 001234"
    
    frame += [ord(c) for c in aprs_payload]

    fcs = crc16_ccitt(frame)
    # Append FCS as little endian
    frame.append(fcs & 0xFF)
    frame.append((fcs >> 8) & 0xFF)

    return frame

def bytes_to_lsb_bits(data):
    bits = []
    for byte in data:
        for i in range(8):
            bits.append((byte >> i) & 1)
    return bits

FLAG = [0,1,1,1,1,1,1,0]    # 0x7E

def ax25_bitstream(frame_bytes):
    bits = bytes_to_lsb_bits(frame_bytes)
    stuffed = bit_stuff(bits)
    bits = [0, 0, 0, 0, 0, 0, 0, 0] * (PREPEND - POSTPEND) + FLAG * POSTPEND + stuffed + FLAG * POSTPEND
    return nrzi_encode(bits)


sine_lut = (32767 * np.sin(2 * np.pi * np.arange(LUT_SIZE) / LUT_SIZE)).astype(np.int16)

# Phase steps
def get_phase_step(freq):
    return int((freq * LUT_SIZE * (1 << 16)) / SAMPLE_RATE)

def generate_bad_afsk_audio(bits):
    samples = []
    phase = 0
    for bit in bits:
        freq = MARK if bit else SPACE
        phase_step = get_phase_step(freq)

        for _ in range(int(SAMPLE_RATE / BIT_RATE)):
            index = (phase >> 16) % LUT_SIZE
            samples.append(sine_lut[index])
            phase += phase_step
    
    return samples


def generate_afsk_audio(bits):
    samples = []
    phase = 0

    bit_index = 0
    bit = bits[bit_index]

    bit_phase = 0
    bit_step = int((BIT_RATE << 16) / SAMPLE_RATE) # Fixed-point step

    for _ in range(int(len(bits) * SAMPLE_RATE / BIT_RATE)):
        freq = MARK if bit else SPACE
        phase_step = get_phase_step(freq)

        index = (phase >> 16) % LUT_SIZE
        samples.append(sine_lut[index])
        phase += phase_step

        bit_phase += bit_step
        if bit_phase >= (1 << 16):
            bit_phase -= (1 << 16)
            bit_index += 1
            if bit_index < len(bits):
                bit = bits[bit_index]
            else:
                break
    
    return samples


# Build the data
frame = build_ax25_frame()
bitstream = ax25_bitstream(frame)

# Generate audio samples
samples = generate_afsk_audio(bitstream)

with wave.open("aprs.wav", "w") as wf:
    wf.setnchannels(1)
    wf.setsampwidth(2)
    wf.setframerate(SAMPLE_RATE)
    wf.writeframes(struct.pack("<{}h".format(len(samples)), *samples))

samples = generate_bad_afsk_audio(bitstream)

with wave.open("aprs_bad.wav", "w") as wf:
    wf.setnchannels(1)
    wf.setsampwidth(2)
    wf.setframerate(SAMPLE_RATE)
    wf.writeframes(struct.pack("<{}h".format(len(samples)), *samples))
