# opus-embedded

An embedded-friendly wrapper around libopus. libopus is compiled to use fixed point math and with
allocator support disable. You need to pass in a buffer to create the decoder in.

## Limitations (as of now)

* no support for floating point or platform specific optimizations where they would be applicable
* no encoder support
* only the most basic functions for decoding are exposed

## Feature flags

### `code-in-ram`

If you are running on a target where access to flash is slow (eg RP2040), you can try enabling the
code-in-ram feature for more performance.
This places a few performance-critical functions in the `.data` section, ie. RAM.

## Performance

A very non-thorough performance test on a Raspberry Pi Pico gives the following:

| `code-in-ram` | stereo | mono |
|---------------|--------|------|
| disabled      | 75%    | 59 % |
| enabled       | 68%    | 51 % |

The parameters for this test were:

* A short, 10s sample encoded to a 48kbps stereo Opus stream. (https://pixabay.com/de/music/intro-outro-good-morning-13396/)
* Decoded to 48kHz, either stero or mono (but note that the Opus stream to be decoded was stereo in both cases)
* Pi Pico running at 125MHz
