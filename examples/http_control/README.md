# http_control

An app whose only input is an HTTP port. The window is filled with one flat color, and another
program on the same machine changes it with a request:

```bash
just dev http_control
curl http://127.0.0.1:8787/color/ff0000    # the window turns red
```

There is no keyboard or mouse handling. The server runs on its own thread and hands each color to
the game through a one-slot mailbox; the game paints the latest one on the next frame, so two
colors sent between frames show only the second.

## Port

The server listens on **127.0.0.1:8787 only** (loopback). It has no authentication and is meant for
local tools, test harnesses and scripts, not for other machines. Set `HTTP_CONTROL_PORT` to use
another port; `0` picks a free one, and the address is printed at startup. If the port is taken,
the window still opens, the console says `NOT LISTENING`, and the color never changes. A value that
is not a port number prints a warning and falls back to 8787. The server is a demo, not a hardened
service: besides binding loopback only, it uses one thread per open connection and has no request
size limit.

## Protocol

The whole command is in the URL path. Any HTTP method works, and a `?query` suffix is ignored.

| Request | Status | Body |
|---|---|---|
| `/color/<code>` | 202 | the accepted color as `rrggbbaa`; it is painted on the next frame |
| `/color` | 200 | the color currently painted, as `rrggbbaa` |
| `/health` | 200 | `ok` |
| `/` | 200 | usage text |
| `/color/<code>` with a code that does not parse | 400 | the reason and the usage text |
| any other path | 404 | the reason and the usage text |

`<code>` is `rgb`, `rrggbb` or `rrggbbaa` in hex, case-insensitive, or one of `black white red
green blue yellow cyan magenta orange gray`. The 3-digit form doubles each digit (`f0c` is
`ff00cc`). A leading `#` is accepted, but a literal `#` cannot be sent in a URL path: encode it as
`%23`.

The alpha byte is accepted and reported back by `GET /color`, but it does not change the window:
the color fills the whole color target, and nothing behind it shows through.

`GET /color` returns the value that was applied, not pixels read back. The code is an sRGB color,
as a color picker shows it. The example makes it the window's clear color: a render feature clears
the color target with it before anything is drawn. The target is linear, so the code is converted
with the exact sRGB curve, and the window surface encodes it back. The window shows the requested
color (`%23336699` shows as `336699`).

## Examples

```bash
curl http://127.0.0.1:8787/                    # usage
curl http://127.0.0.1:8787/health              # ok
curl http://127.0.0.1:8787/color/00ff00        # green
curl http://127.0.0.1:8787/color/f0c           # ff00cc
curl http://127.0.0.1:8787/color/orange        # ff8800
curl 'http://127.0.0.1:8787/color/%23336699'   # #336699
curl -X POST http://127.0.0.1:8787/color/blue  # the method does not matter
curl http://127.0.0.1:8787/color               # 0000ffff
curl -i http://127.0.0.1:8787/color/teal       # 400
curl -i http://127.0.0.1:8787/colour/ff0000    # 404
```
