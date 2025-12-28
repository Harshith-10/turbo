# Turbo API & CLI Reference

This document provides a comprehensive reference for the Turbo project, including the Server HTTP API and the Command Line Interface (CLI).

## Table of Contents
1. [Server HTTP API](#server-http-api)
    - [Execute Code](#execute-code)
    - [Get Runtimes](#get-runtimes)
    - [Get Packages](#get-packages)
    - [Data Models](#data-models)
2. [Command Line Interface (CLI)](#command-line-interface-cli)
    - [Global Options](#global-options)
    - [Commands](#commands)

---

## Server HTTP API

The Turbo Server exposes a RESTful API for executing code in a isolated, sandboxed environment.

**Base URL**: `/api/v1`

### Execute Code

Executes a job consisting of source files, optional test cases, and configuration.

- **URL**: `/api/v1/execute`
- **Method**: `POST`
- **Content-Type**: `application/json`

#### Request Body
The request must strictly follow the `JobRequest` schema.

```json
{
  "language": "python",
  "version": "3.10",
  "files": [
    {
      "name": "main.py",
      "content": "print('Hello World')",
      "encoding": "utf8"
    }
  ],
  "testcases": [
    {
      "id": "1",
      "input": "test input",
      "expected_output": "Hello World\n"
    }
  ],
  "args": ["arg1"],
  "stdin": "standard input",
  "run_timeout": 3000,
  "compile_timeout": 5000,
  "run_memory_limit": 536870912,
  "compile_memory_limit": 536870912
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `language` | string | **Yes** | The programming language (e.g., "python", "cpp", "rust"). |
| `version` | string | No | Specific version of the language (e.g., "3.10"). |
| `files` | array | **Yes** | List of `FileRequest` objects. |
| `testcases` | array | No | List of `Testcase` objects for grading. |
| `args` | array | No | Command line arguments for the program. |
| `stdin` | string | No | Standard input (used if no testcases are provided). |
| `run_timeout` | integer | No | Execution timeout in milliseconds (default: 3000). |
| `compile_timeout` | integer | No | Compilation timeout in milliseconds (default: 3000). |
| `run_memory_limit` | integer | No | Memory limit for execution in bytes (default: 512MB). |
| `compile_memory_limit` | integer | No | Memory limit for compilation in bytes. |

#### Response Body
Returns a `JobResult` object.

```json
{
  "language": "python",
  "version": "3.10.12",
  "run": {
    "status": "Success",
    "stdout": "Hello World\n",
    "stderr": "",
    "exit_code": 0,
    "memory_usage": 10240,
    "cpu_time": 5000,
    "execution_time": 100
  },
  "compile": null,
  "testcases": []
}
```

### Get Runtimes

Retrieves the list of supported languages and runtimes available on the server.

- **URL**: `/api/v1/runtimes`
- **Method**: `GET`

#### Response
Returns an array of `Runtime` objects.

```json
[
  {
    "language": "python",
    "version": "3.10.12",
    "aliases": ["py", "python3"],
    "runtime": "python3"
  }
]
```

### Get Packages

Retrieves the list of installed packages/libraries availability.

- **URL**: `/api/v1/packages`
- **Method**: `GET`

#### Response
Returns an array of `Package` objects.

```json
[
  {
    "language": "python",
    "language_version": "3.10.12",
    "installed": true
  }
]
```

### Data Models

#### FileRequest
| Field | Type | Description |
|---|---|---|
| `name` | string | Filename (e.g., `main.py`). |
| `content` | string | The file content. |
| `encoding` | string | `utf8`, `base64`, or `hex` (default: `utf8`). |

#### StageResult (Run/Compile)
| Field | Type | Description |
|---|---|---|
| `status` | string | `Pending`, `Running`, `Success`, `RuntimeError`, `CompilationError`, `TimeLimitExceeded`, `MemoryLimitExceeded`, `OutputLimitExceeded`. |
| `stdout` | string | Standard output. |
| `stderr` | string | Standard error. |
| `exit_code` | integer | Process exit code. |
| `memory_usage` | integer | Peak memory usage in bytes. |
| `cpu_time` | integer | CPU time utilized in nanoseconds. |
| `execution_time` | integer | Wall-clock time in milliseconds. |

---

## Command Line Interface (CLI)

The `turbo` CLI allows you to interact with the Turbo Server and manage packages directly from your terminal.

**Usage**: `turbo [COMMAND]`

### Global Environment Variables
- `TURBO_HOME`: Overrides the default home directory (default: `~/.turbo`).
- `TURBO_PACKAGES_PATH`: Path to the local package repository (default: `./packages`).

### Commands

#### `execute`
Execute a source file on the Turbo Server.

```bash
turbo execute [OPTIONS] <FILE> <LANGUAGE>
```

**Arguments:**
- `<LANGUAGE>`: The programming language of the file (e.g., `python`, `rust`).
- `<FILE>`: Path to the source file to execute.

**Options:**
- `--version <VERSION>`: Specify the language version.
- `--server <URL>`: URL of the Turbo Server (default: `http://localhost:3000`).

**Example:**
```bash
turbo execute python main.py --server http://localhost:3000
```

#### `pkg`
Manage packages and their installations (if applicable to the environment).

**Subcommands:**
- `install`
  - **Usage**: `turbo pkg install <NAME> [OPTIONS]`
  - **Options**:
    - `--version <VERSION>`: Version to install.
    - `--local <PATH>`: Install from a local path.
- `list`
  - **Usage**: `turbo pkg list`
  - **Description**: List all available packages in the repository.

#### `cache`
Manage local cache.

**Subcommands:**
- `clear`
  - **Usage**: `turbo cache clear`
  - **Description**: Clears the compilation/execution cache (default: `/tmp/turbo-cache`).

#### `start`
Start the Turbo Server (currently a placeholder wrapper).

```bash
turbo start
```

---

_Generated by Antigravity Agent_
