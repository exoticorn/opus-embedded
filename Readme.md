# opus-embedded

An embedded-friendly wrapper around libopus. libopus is compiled to use fixed point math and with
allocator support disable. You need to pass in a buffer to create the decoder in.

## Limitations (as of now)

* no support for floating point or platform specific optimizations where they would be applicable
* no encoder support

## Feature flags

### `code-in-ram`

If you are running on a target where access to flash is slow (eg RP2040), you can try enabling the
code-in-ram feature for more performance.
This places a few performance-critical functions in the `.data` section, ie. RAM. (At the cost
of some assembler warnings.)

## Performance

Here are some performance numbers on a Raspberry Pi Pico running at the default of 125MHz:

| sample    | kbps | `code-in-ram` | stereo | mono |
|-----------|------|---------------|--------|------|
| jingle    | 48   | off           | 75%    | 59%  |
|           |      | on            | 68%    | 51%  |
| rock      | 96   | off           | 83%    | 66%  |
|           |      | on            | 80%    | 66%  |
| audiobook | 32   | off           | 77%    | 60%  |
|           |      | on            | 67%    | 52%  |

The percentages are decoding time in relation to the duration of the sample. This needs to be
safely below 100% for real-time playback.
