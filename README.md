# Pico APRS Beacon

## Core Purpose

This device is meant to display GPS info and send APRS position beacons on a
user-defined schedule.

I want this device to be stable, and to perform its intended purpose without
trouble while portable. I should be able to hook it up to my HT and have it send
APRS position reports on a schedule, with some fun stuff to look at on the
screen. I'd like to test it out by taking it with me on bike rides, or when
going on longer drives.

## Minimum Viable Features

- [x] Parse and extract fix data from NMEA.
- [x] Display: Latitude, Longitude, Altitude, Fix status.
- [x] APRS encoder: create position beacons in AX.25.
- [x] Bell 202 AFSK modulator: drive output via I2S DAC.
- [ ] Persist location data in case fix is lost.
- [x] Timer to determine when to send a beacon.
- [ ] Flexible enough to try again multiple times if a GPS fix isn't current enough.

## Architecture Goals

- No `unsafe` or `static mut` unless absolutely necessary.
- Application logic must stay in safe Rust.
- Avoid heap allocations.
- Ideally, the main application logic would read like a normal program running
  on an operating system. Clean separation of hardware related things and logic.

## Stretch Goals

- [ ] Menu system: Allow configuration of digipeater paths, call, SSID. I have a
  rotary encoder that could work.
- [ ] Map display: Be able to draw logged position data scaled down to a line path.
- [ ] Beacon compression (Mic-E)
- [ ] APRS RX: Listen for and display decoded packets on the screen. Maybe even
  chart them down on a map.
