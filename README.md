# miniclaw

> Your personal AI agent, optimized for edge computing

miniclaw is a lightweight AI agent that runs on Raspberry Pi and edge devices. At under 15 MB with 256 MB RAM, it provides complete AI capabilities without requiring the cloud.

## Features

- **Ultra-lightweight**: Binary < 15 MB, runs on 256 MB RAM
- **Multiple providers**: OpenAI, Ollama (local models)
- **Persistent memory**: Remembers conversations long-term
- **Daemon mode**: Runs in background with session management
- **Telegram integration**: Chat with your agent via Telegram
- **Skill system**: Extensible via custom skills
- **Built-in tools**: Filesystem, web, command execution, cron

## Installation

### Prerequisites

- Rust 1.85+ (or prebuilt binary)
- An OpenAI API key or Ollama installed

### Install via cargo

```bash
cargo install miniclaw
```

### Install from source

```bash
git clone https://github.com/mattdef/miniclaw.git
cd miniclaw
cargo build --release

# Binary is in target/release/miniclaw
sudo cp target/release/miniclaw /usr/local/bin/
```

## Quick Start

### 1. Initialize

```bash
miniclaw onboard
```

This creates the workspace in `~/.miniclaw/` with configuration files.

### 2. Configure

Create a file `~/.miniclaw/config.json`:

```json
{
  "provider": "ollama",
  "model": "llama3.2:1b",
  "ollama_url": "http://localhost:11434"
}
```

Or use OpenAI:

```json
{
  "provider": "openai",
  "model": "gpt-4o-mini",
  "openai_api_key": "sk-..."
}
```

### 3. First message

```bash
miniclaw agent -m "Hello, who are you?"
```

## Usage

### Interactive mode (one-shot)

Send a single message and get a response:

```bash
miniclaw agent -m "Explain Rust programming"
```

With a specific model:

```bash
miniclaw agent -M "google/gemini-2.5-flash" -m "What's the weather like?"
```

### Daemon mode (gateway)

Launch the daemon for persistent sessions:

```bash
miniclaw gateway
```

The gateway manages:

- User sessions
- Message routing (Telegram, CLI)
- Automatic persistence every 30 seconds

### Memory management

View today's memories:

```bash
miniclaw memory read
```

View all history:

```bash
miniclaw memory read --long
```

Search memories:

```bash
miniclaw memory rank -q "rust project"
```

View last 7 days of notes:

```bash
miniclaw memory recent
```

### Global options

```bash
# Verbose mode
miniclaw --verbose agent -m "Debug this code"

# Custom config file
miniclaw --config /path/config.json agent -m "Hello"

# Default model override
miniclaw --model "llama3.2:1b" agent -m "Test"
```

## Architecture

```
miniclaw/
├── Agent Loop         # Main agent loop
├── Chat Hub          # Message routing
├── Memory Store      # Long and short-term memory
├── Tool Registry     # Available tools (filesystem, web, exec)
├── Session Manager   # Session management
├── Skills Manager    # Extensible skill system
└── Providers         # LLM abstraction (OpenAI, Ollama)
```

## Available Commands

| Command   | Description                        |
| --------- | ---------------------------------- |
| `onboard` | Initialize workspace               |
| `agent`   | Send one-shot message to agent     |
| `gateway` | Launch background daemon           |
| `memory`  | Manage memory (read, recent, rank) |
| `version` | Display version                    |

## Advanced Configuration

### Environment variables

```bash
export RUST_LOG=debug          # Log level
export OPENAI_API_KEY=sk-...   # OpenAI API key
```

### Complete configuration file

```json
{
  "provider": "ollama",
  "model": "llama3.2:1b",
  "ollama_url": "http://localhost:11434",
  "telegram_bot_token": "...",
  "telegram_chat_id": "...",
  "workspace_path": "/custom/path"
}
```

## Why miniclaw?

- **Private**: Your data stays on your machine
- **Economical**: No API costs with Ollama
- **Fast**: Instant response time locally
- **Reliable**: Works offline with Ollama
- **Scalable**: Modular and extensible architecture

## License

MIT

---

Built with Rust, for edge computing.
