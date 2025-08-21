## Logger

A small logging library with `debug!`, `log!`, `warn!`, `error!`, `new_line!` macros (same signature as `println!`).

### Features
- Colored level badge: colored background + black label text (`DBG`, `LOG`, `WRN`, `ERR`).
- Timestamp in format `YYYY.mm.dd HH:MM:SS.ms`, printed in the same color as the base text color for the level.
- Default message color depends on the level:
  - `debug` → gray, `info` → blue & white, `warn` → orange, `error` → red.
- Inline markup for local coloring and styles: `<tokens>Text</>`.
  - Tokens can be in any order.
  - Supported: one color + `italic` | `i`, `bold` | `b`, `underline` | `u`.

## Installation and usage

In your project's `Cargo.toml`:
```toml
[dependencies]
logger = { path = "../logger" }
```

In code:
```rust
logger::debug!("Debug message");
logger::log!("User {} logged in", user_id);
logger::warn!("<yellow,italic>Low disk</>: {:.1}%", percent);
logger::error!("<red,bold>Failed</>: {}", err);

// Setup aliases for common styling patterns
logger::setup!("#", "purple,i");
logger::setup!("!", "yellow,b");
logger::setup!("*", "red,bold,underline");

// Use aliases in messages
logger::log!("<#>Component name</>: Load resource <!>en-US</>");
logger::warn!("<*>Critical warning</>: System overload detected");
```

## Log levels (build-time)
The level is chosen at build time via the `LOG_LEVEL` variable (read from `.env` or the environment during build).

Allowed values and what gets printed:
- `debug`: debug, log, warn, error
- `info`: log, warn, error
- `warn`: warn, error
- `error`: error
- `none`: nothing

### Configure via .env
Create a `.env` file at the project root:
```env
LOG_LEVEL=info
```

Build/run:
```bash
cargo build
cargo run
```

Notes:
- The build script watches `.env` — changes trigger a rebuild.
- You can set `LOG_LEVEL` directly in the environment during build (e.g., in CI).

## Message styling markup
Syntax: `<tokens>Text</>`

- Tokens (order does not matter):
  - Styles: `italic`, `bold`, `underline`
  - Color (exactly one from the list below)
- Markup applies only to the content between `<...>` and `</>`.
- Markup does not support nesting.

Examples:
```rust
logger::log!("Hello, <red,bold>world</>!");
logger::warn!("<yellow,italic,underline>Low battery</>: {}%", 7);
logger::debug!("Mix <gray,italic>and</> match");
```

### Available colors
  - `black`
  - `red`
  - `green`
  - `orange`/`yellow`
  - `blue`
  - `purple`/`magenta`
  - `cyan`
  - `white`
  - `gray`

Examples:
```rust
logger::log!("<blue>info</>");
logger::warn!("<yellow,underline>warning</>");
logger::error!("<red,bold>error</>");
logger::debug!("<gray,italic>debug</>");
```

## Style aliases
For frequently used styling patterns, you can create aliases using the `setup!` macro:

```rust
// Register aliases
logger::setup!("#", "purple,i");      // Component names
logger::setup!("!", "yellow,b");      // Important values
logger::setup!("*", "red,bold,underline"); // Critical warnings
logger::setup!("@", "cyan,italic");   // Debug info

// Use aliases in messages
logger::log!("<#>UserService</>: Processing <!>user123</>");
logger::warn!("<*>Database connection failed</>: Retrying...");
logger::debug!("<@>Cache miss</>: Loading from database");
```

Aliases are resolved at runtime and can contain any valid styling tokens (colors, bold, italic, underline).

## Log line format
General view:
```
[BG] LVL [BG_CLEAR] [YYYY.mm.dd HH:MM:SS.ms] [FONT]Message[FONT_CLEAR]
```

Where `[BG]`/`[BG_CLEAR]` and `[FONT]`/`[FONT_CLEAR]` are the corresponding ANSI sequences.

## Terminal support
- Colors are ANSI escape codes. Modern Windows terminals (Windows Terminal), Linux and macOS support this by default.
- If colors are not visible in older consoles, use an ANSI-capable terminal.
