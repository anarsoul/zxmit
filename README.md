# zxmit

## Description

esxDOS dot command & PC utility to upload arbitrary files over the air from PC to a WiFi equipped ZX Spectrum

It contains two counterparts: Speccy part and PC part.

### ZX Spectrum part

ZX Spectrum dot-command starts a TCP server that listens on port 6144.

It accepts connection, receives the data in 1k blocks with 17-byte header and saves it in the current directory.

Once you run the command, it will show IP address of server and port. You will need the address to specify it in the PC utility.

Run `.zxmit` from BASIC to start the utility

### PC part

There are 2 flavors: CLI and GUI, both written in Rust. GUI is self-explanatory, CLI usage is:

`zxmit <IP> filaname`

Run `zxmit -h` for a full list of command line arguments

## Legal

This software is licensed under MIT license

(c) 2022 Alex Nihirash
(c) 2025 Vasily Khoruzhick

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
