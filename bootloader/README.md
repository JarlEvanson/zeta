# Logging
The bootloader has the capability of logging any errors or useful information by using a serial port or by writing the output to a framebuffer.

### Behavior Pre-Config
Since the logging system is initialized before parsing the configuration file, the bootloader must have defaults that are used before loading the user desired logging configuration from the configuration file. 

Filtering of logs before the configuration file is loaded can be controlled by setting these environment variables:
- `PRECONFIG_GLOBAL`
    - Controls the filtering of all messages.
- `PRECONFIG_SERIAL`
    - Controls the filtering of all messages logged through the serial output.
- `PRECONFIG_FRAMEBUFFER`
    - Controls the filtering of all messages logged onto the framebuffer.

These environment variables accept `off`, `error`, `warn`, `info`, `debug`, or `trace` as valid commands. When they are not set, their values default to `error`.