# Logging
The bootloader has the capability of logging any errors or useful information by using a serial port or by writing the output to a framebuffer.

Use of the serial and framebuffer logging methods are controlled by the `serial_logging` and `framebuffer_logging` features respectively. They are on by default.

### Behavior Pre-Config
Since the logging system is initialized before parsing the configuration file, the bootloader must have defaults that are used before loading the user desired logging configuration from the configuration file. 

Filtering of logs before the configuration file is loaded can be controlled by setting these environment variables:
- `PRECONFIG_GLOBAL`
    - Controls the filtering of all messages.
- `PRECONFIG_SERIAL`
    - Controls the filtering of all messages logged through the serial output.
- `PRECONFIG_FRAMEBUFFER`
    - Controls the filtering of all messages logged onto the framebuffer.

Valid values of the above variables are `off`, `error`, `warn`, `info`, `debug`, or `trace`. When they are not set, they default to `error`.