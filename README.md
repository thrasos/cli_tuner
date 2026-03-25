# cli_tuner

CLI microphone tuner for bouzouki and classic guitar in Rust.

`cli_tuner` listens to your default system microphone, estimates the played pitch in real time, and prints the nearest target string with the cents offset.

## Features

- Supports `tetrachordo` tuning: `C3 F3 A3 D4`
- Supports `trichordo` tuning: `D3 A3 D4`
- Supports `classic-guitar` tuning: `E2 A2 D3 G3 B3 E4`
- Uses the default system microphone
- Prints the nearest target string, frequency, and cents offset
- Supports adjustable concert A reference with `--reference`
- Can print tuning targets without opening the microphone via `--list`

## Requirements

- Rust and Cargo installed
- A working input microphone
- Microphone permission granted to your terminal app on macOS

Check your Rust toolchain:

```bash
rustc --version
cargo --version
```

## Installation

Clone the repository and build it:

```bash
git clone <your-repo-url>
cd cli_tuner
cargo build --release
```

Run the release binary directly:

```bash
./target/release/cli_tuner --tuning trichordo
```

You can also run it without a release build:

```bash
cargo run -- --tuning trichordo
```

## How To

1. Tune your instrument in a quiet room.
2. Place the microphone reasonably close to the instrument.
3. Start the tuner with the tuning preset you want.
4. Pluck one string or course at a time.
5. Read the output: detected frequency in Hz, nearest target string, cents offset, and whether to tune up, tune down, or hold the note as in tune.

Example output:

```text
312.88 Hz | D4     high Re  | +109.7 cents | tune down
294.22 Hz | D4     high Re  |   +3.3 cents | in tune
```

If the output says `tune up`, the detected pitch is flat.

If the output says `tune down`, the detected pitch is sharp.

## Supported Tunings

### `tetrachordo`

- `C3`
- `F3`
- `A3`
- `D4`

### `trichordo`

- `D3`
- `A3`
- `D4`

### `classic-guitar`

- `E2`
- `A2`
- `D3`
- `G3`
- `B3`
- `E4`

## Usage

Show help:

```bash
cargo run -- --help
```

Start the tuner with tetrachordo:

```bash
cargo run -- --tuning tetrachordo
```

List string targets without opening the microphone:

```bash
cargo run -- --tuning trichordo --list
```

Classic guitar example:

```bash
cargo run -- --tuning classic-guitar
```

Use a different concert A reference:

```bash
cargo run -- --tuning classic-guitar --reference 442
```

## Command-Line Options

- `--tuning`, `-t`: Selects the tuning preset
- `--reference`, `-r`: Sets concert A in Hz, from `400` to `490`
- `--list`: Prints target strings and exits
- `--help`, `-h`: Prints help

## Notes

- The tuner uses the default system input device.
- Pitch detection is optimized for approximately `70 Hz` to `550 Hz`.
- Best results come from single notes or courses played clearly, without heavy background noise.

## Troubleshooting

### No microphone input detected

- Check that your microphone is connected and selected as the default input device.
- Make sure no other app is blocking access to the microphone.
- On macOS, enable microphone access for your terminal in System Settings > Privacy & Security > Microphone.

### Pitch jumps or looks unstable

- Move closer to the microphone.
- Reduce background noise.
- Pluck only one string or course at a time.
- Let the note ring clearly before adjusting the tuning peg.

## Development

Run the test suite:

```bash
cargo test
```
