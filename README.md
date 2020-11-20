# wcursorgen

This program reads the config file to find the list of cursor x2 in PNG format along with their hotspot and nominal size, then converts all of the x2 to CUR or ANI format.

The config file format is the same with xcursorgen. Each line in the config is of the form:

```
<size> <x-hot> <y-hot> <filename> <ms-delay>
```

Multiple images with the same `<size>` are used to create animated cursors, the `<ms-delay>` value on each line indicates how long each image should be displayed before switching to the next. `<ms-delay>` can be elided for static cursors.

Note: on Windows, the frame rate of animated cursor is in terms of jiffies (1/60 sec), so the difference of `<ms-delay>` will not take effect precisely. For example, both `30 ms` and `40 ms` result in `round(30 / 16.667) = round(40 / 16.667) = 2 jiffies` in the generated cursor file.

## Usage

```
USAGE:
    wcursorgen.exe [OPTIONS] --config <config> --output <output> --size <size>

FLAGS:
    -h, --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -c, --config <config>
            The path of config file

    -o, --output <output>
            The path of output file without file ext (a .cur or .ani ext will be automatically
            appended according to whether the cursor is animated)

    -p, --prefix <prefix>
            Find cursor x2 in the directory. If not specified, the current directory is used

    -s, --size <size>
            Choose which size to generate. Unlike xcursor, one ANI file cannot contain multiple x2
            in different sizes, so we must pick up one. The size specified must exist in the config
```
