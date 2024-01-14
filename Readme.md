# opus-embedded

An embedded-friendly wrapper around libopus. libopus is compiled to use fixed point math and with
allocator support disable. You need to pass in a buffer to create the decoder in.

## Limitations (as of now)

* no support for floating point or platform specific optimizations where they would be applicable
* no encoder support
* only the most basic functions for decoding are exposed
