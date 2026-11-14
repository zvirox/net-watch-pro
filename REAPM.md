# REAPM.md — Desktop Application Firewall: Analysis Report

**Subject:** Desktop application firewall and privacy suite (installed on this machine)  
**Version analyzed:** 2.2.9 dev tree + installed 2.2.8 binary  
**Build:** Go 1.26.0 toolchain go1.26.3; Rust 1.89; Angular 16; Tauri 2.2.5  
**Analysis date:** 2026-11-14  
**License:** GPLv3  
**Analysis method:** Static only — source-tree reconnaissance plus direct binary/string/config inspection of the installed build. No runtime observation; the core service was NOT started (it would take over the host network stack).

---

## Contents

1. [What It Is](#what-it-is)
2. [Installation Layout](#installation-layout)
3. [How the Two Binaries Cooperate](#how-the-two-binaries-cooperate)
4. [Core Daemon Architecture](#core-daemon-architecture)
5. [Configuration Registry](#configuration-registry)
6. [API Surface](#api-surface)
7. [Intel / Threat Data](#intel-threat-data)
8. [UI Shell Architecture](#ui-shell-architecture)
9. [UI Views / Routes](#ui-views-routes)
10. [Traffic Protection Workflow](#traffic-protection-workflow)
11. [Privacy Overlay Network (SPN)](#privacy-overlay-network-spn)
12. [System Service & Security Model](#system-service-security-model)
13. [Key Dependencies](#key-dependencies)
14. [Component Inventory Summary](#component-inventory-summary)
15. [MCP / LLM / AI Integration](#mcp-llm-ai-integration)
16. [Evidence & Limitations](#evidence-limitations)
17. [Unknowns](#unknowns)

---

## 1. What It Is

A free, open-source application firewall and privacy suite. It intercepts all outbound network traffic on the host, matches connections to applications via process attribution, applies per-app rules plus global filter lists, logs every connection, and optionally tunnels allowed traffic through a privacy overlay network. Everything runs locally on the device except the optional overlay network.

Design principles visible in the code:
- All settings local to the device (except overlay traffic)
- Defaults aim to require no effort; advanced users can configure down to per-app rules
- Separation: core service runs in system context (root), UI elements run in user context
- Automatic signed updates and auto-downloaded intelligence data (block lists, geoip)

---

## 2. Installation Layout

| Path | Role |
|------|------|
| `/usr/bin/app` | Launcher script → UI shell; sets `WEBKIT_DISABLE_COMPOSITING_MODE=1` |
| `/usr/lib/app/core` | 45 MB Go ELF — daemon doing packet interception |
| `/usr/lib/app/app` | 27 MB Rust ELF — Tauri UI shell |
| `/usr/lib/app/app.zip` | 2 MB Angular UI bundle, served by core |
| `/usr/lib/app/assets.zip` | Favicons/icons served at `/assets/` |
| `/usr/lib/systemd/system/app.service` | systemd unit, enabled, restarts on failure |
| `/var/lib/app/intel/` | Auto-updated intel: filter lists, GeoIP MMDBs, overlay map, notifications, news |

Systemd unit excerpts:
- `Before=nss-lookup.target network.target shutdown.target`, `After=systemd-networkd.service`
- `Conflicts=shutdown.target firewall-service`, `Wants=nss-lookup.target`
- `EnvironmentFile=-/etc/default/app`
- `ExecStart=/usr/lib/app/core --log-dir=/var/lib/app/log -- $APP_ARGS`
- `ExecStopPost=-/usr/lib/app/core -recover-iptables`
- Sandboxing: `MemoryDenyWriteExecute=yes`, `PrivateTmp=yes`, `ProtectSystem=true`, `ProtectHome=read-only`, `RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_INET AF_INET6`
- Capability bounding set: `cap_chown cap_kill cap_net_admin cap_net_bind_service cap_net_broadcast cap_net_raw cap_sys_module cap_sys_ptrace cap_dac_override cap_fowner cap_fsetid cap_sys_resource cap_bpf cap_perfmon`

Journal observation (previous boot): started, terminated by signal, stopped cleanly; peak memory 322.2 MB, CPU ~1 min 26 s over 5 min 41 s wall.

---

## 3. How the Two Binaries Cooperate

1. **Core** boots via systemd, loads config + intel, starts its HTTP/WebSocket API on `127.0.0.1:817` (`/api/v1/…`), takes over the network stack (nfqueue/conntrack/eBPF/iptables), serves the Angular UI from its embedded HTTP server at `/ui/modules/app/`.
2. **UI shell** (`app-ui-start.sh` → `app`): checks WebKit availability, manages the systemd service (`systemctl` with `pkexec`/`gksudo` fallback), creates splash window → main window titled "App", loads `http://127.0.0.1:817/api/v1/ui/modules/app/`, adds a tray icon with Pause/Resume (5/15/60 min), overlay toggles, and "Exit UI — this does not stop the system service."
3. Tray exit confirms: "Do you really want to quit the user interface? This does not stop the App system service."
4. Upgrade path: core downloads a new UI zip, relaunches the UI process; if relaunch fails, continues with the current UI process.

The web UI works standalone in any browser at `http://127.0.0.1:817`; the Tauri shell adds window chrome, tray, notifications, service control.

---

## 4. Core Daemon Architecture

### 4.1 Build identity
- Module path: `cmds/app-core`
- Version stamped: `v2.2.8`, build source git, build time `2026-10-27T18:29:12Z`, VCS revision `5c820354…` (clean)
- `CGO_ENABLED=0`, `GOOS=linux`, `GOARCH=amd64`, `GOAMD64=v1`
- Full pure-Go dependency set visible in module metadata

### 4.2 Package families
`base/` — platform foundation: `api`, `config`, `database` (bbolt + SQLite/zombiezen storage, migrations), `info`, `log`, `metrics`, `notifications`, `rng`, `runtime`, `utils`, `utils/debug`, `template`, `container`, `apprise`

`service/` — feature modules: `api`, `broadcasts`, `compat`, `configure`, `control`, `core`, `detection`, `firewall`, `integration`, `intel`, `interop`, `mgr`, `nameserver`, `netenv`, `netquery`, `network`, `process`, `profile`, `resolver`, `splittun`, `status`, `sync`, `ui`, `updates`

`spn/` — overlay network packages: `access`, `cabin`, `captain`, `conf`, `crew`, `debug`, `docks`, `hub`, `instance`, `navigator`, `patrol`, `ships`, `sluice`, `spn`, `terminal`, `test`, `testing`, `tools`, `unit`

`cmds/` — `cmdbase` (Linux system service wrapper), `app-core` (main), `hub`, `integrationtest`, `observation-hub`, `portmaster-core`, `testsuite`, `trafficgen`, `updatemgr`, `winkext-test`

`desktop/` — `angular` (UI), `tauri` (Rust shell)

### 4.3 Module system (`service/mgr`)
- Multi-stage, dependency-aware boot: each module has `prep()` (flags/config registration), `start()` (tasks/workers), `stop()` (goroutine cleanup)
- `Group` and `ExtendedGroup` coordinate module lifecycle with state managers
- Event/hook/state managers (`EventMgr[T]`, `HookMgr`) with typed pub/sub
- `SleepyModule` interface: `SetSleep(bool)` applies sleep mode across all modules in main + overlay groups
- Interception modules grouped into a pausable/interruptible group
- Restart via exit code 23; shutdown with 10 ms propagation delay for event delivery

---


---

## 5. Configuration Registry

Exported via `core --export-config-options`. Options are declared with metadata: name, key, description, OptType (string/string-array/int/bool/any), ExpertiseLevel (user/expert/developer), ReleaseLevel (stable/beta/experimental), RequiresRestart, DefaultValue, ValidationRegex, PossibleValues, and UI annotations (`category`, `display-hint`, `display-order`).

Three-layer priority: user configuration → default configuration → fallback values supplied at registration. Config variable accessors are fast when unchanged; a concurrent wrapper exists for goroutine safety. Release-level gating hides beta/experimental options from stable builds.

Sample options:
- `core/expertiseLevel` — UI Mode: `user` (Simple Interface, hide complex settings), `expert` (Advanced Interface), `developer` (Developer mode). Default `user`; no restart required; category "User Interface", one-of display hint, order -16.
- `core/releaseLevel` — Feature Stability: `stable`, `beta`, `experimental`. Default `stable`; category "Updates", order -8.
- `core/useSystemNotifications` — Desktop Notifications; bool, user level, default true, category "User Interface", order -15.

---

## 6. API Surface

Base: `http://127.0.0.1:817/api/v1/` — HTTP + WebSocket, session-cookie + bearer/basic auth, client allow-list via `--allowed-clients`.

Permission model (api.Permission): `NotFound(-2)`, `Dynamic(-1)`, `NotSupported(0)`, `PermitAnyone(1)`, `PermitUser(2)`, `PermitAdmin(3)`, `PermitSelf(4)`. Write implicitly includes read. Session cookie TTL 5 minutes; API key authentication supported; `app/auth` endpoint lets external tools request a time-limited token via a user-confirmation prompt ("An app requests access to the App"). Endpoints declare Read/Write permissions; the API server enforces them per request.

Core endpoints (registered early, before module registration): `auth/basic`, `auth/bearer`, `auth/permissions`, `auth/reset`, `config/options`, `debug/{allocs,cpu,heap,info,stack}`, `debug/stack/print`, `endpoints`, `ping`, `ready`, `updates/check`, `updates/apply`, `updates/get/{path}/{name}`, `core/shutdown`, `core/restart`, `core/paths`, `app/auth`, `app/profile`, `ui/reload`.

Module endpoints registered at full startup (examples): `profile/merge`, `profile/icon`, `customlists/update`, `intel/geoip/countries`, overlay network endpoints under `spn/…`. Modules self-register via `api.RegisterEndpoint(api.Endpoint{…})` with `Path`, `Read`/`Write`/`ActionFunc`/`StructFunc`/`DataFunc`/`HandlerFunc`, `Name`, `Description`, `Parameters`.

An endpoint-bridge storage injects the API into the database subsystem so API calls can be made as database record operations (Get/Put keyed as `api:{path}`), subject to a scope check that the path stays under `/api/v1/`.

Debug endpoints expose pprof profiles (allocs/cpu/heap), goroutine stack, version/platform info, and goroutine dumps. Metrics use VictoriaMetrics-compatible format; Prometheus push-gateway URL configurable.

---

## 7. Intel / Threat Data

Stored in `/var/lib/app/intel/`, auto-updated from update index URLs:

| File | Size | Purpose |
|------|------|---------|
| `index.json` | ~4 KB | Intel index: name, version `20261114.3.7`, published timestamp, artifact manifest with SHA-256 + source URLs |
| `main-intel.yaml` | ~15 KB | Overlay bootstrap hubs (observed: DE nodes tcp/17,80,8080,17027,17080; FI nodes http/80 tcp/17027; US hub in Ashburn VA, Trusted, Verified) |
| `notifications.yaml` | ~15 KB | User-facing notifications |
| `news.yaml` | ~2 KB | News cards |
| `index.dsd` | ~4 KB | Filter-list index (DSD format) |
| `base.dsdl` | 8.3 MB | Base filter lists (DSDL msgpack) |
| `intermediate.dsdl` | ~1 MB | Intermediate filter lists |
| `urgent.dsdl` | ~25 KB | Urgent filter lists |
| `geoipv4.mmdb` / `geoipv6.mmdb` | ~53 MB each | MaxMind GeoIP databases |

Filter-list lookups support domain (incl. subdomains via publicsuffix), IP, ASN, and country-code matching. Custom user-maintained filter lists reload on file change and via `customlists/update` endpoint; they are applied in the connection verdict pipeline alongside base lists.

---

## 8. UI Shell Architecture

**Binary:** Rust ELF, 27 MB, Tauri 2.2.5 with wry/WebKitGTK, Earthly-built (Rust 1.89), patched Safing fork of Tauri/tao (Windows-only window-manager patch). Plugins: tray-icon, image-png, config-json5, devtools, shell, dialog, clipboard-manager, os, single-instance, notification, log, window-state, websocket.

Key behaviors:
- WebKit availability check; aborts with "Webkit is not installed" if missing
- Service management via `systemctl` (checked at `/sbin`, `/bin`, `/usr/sbin`, `/usr/bin`), with `pkexec`/`gksudo` sudo fallback
- Window lifecycle: splash window → main window labeled "app::window" titled "App"; navigation blocked to `about:blank`; `TAURI_SHOW_IMMEDIATELY` env override
- Tray menu ("app::traymenu"): global status, overlay status, Pause overlay 5/15/60 min, Pause app 5/15/60 min, Resume now, Disable/Enable overlay, Exit UI
- Notifications plugin: title/body/summary/actions/icon, system notifications from the API notification stream via websocket subscription (`query notifications:`)
- Custom Tauri commands exposed to JS: `should_show`, `should_handle_prompts`, `get_state`/`set_state` (arbitrary key-value store), `get_app_info` (Linux via XDG; unsupported on Windows), `get_service_manager_status`
- Persistent websocket to `http://127.0.0.1:817/api/v1/` via a custom portapi client; handles connect/disconnect handlers, pending UI restart on upgrade
- Single-instance enforcement; window-state persistence

The frontend talks to core over HTTP + websocket database subscriptions (`PortapiService` in `@safing/portmaster-api` library). UI state (intro finished, sort order, etc.) is persisted under key `core:ui/v1` in the API database.

---

## 9. UI Views / Routes

Angular 16.2.12 SPA, webpack chunks `runtime`/`polyfills`/`scripts`/`vendor`/`main`. UI libraries: `ng-zorro-antd` components, FontAwesome 6 icons, d3 charts, fuse.js fuzzy search, ngx-markdown. Base href `/ui/modules/app/`.

Routes:
- `` → redirect `dashboard`
- `dashboard` — dashboard (default landing)
- `settings` — settings (gated by expertise level)
- `app` → redirect `app/overview`
- `app/overview` — application overview (app list)
- `app/:source/:id` — per-application detail/profile view
- `monitor` — connection monitor
- `monitor/profile/:source/:profile` → redirect `monitor`
- `support` — support/diagnostics
- `support/:id` — support form
- `spn` — overlay network status/controls
- `**` → redirect `dashboard`

Pages/components: app-view, dashboard, monitor, settings, spn, support (+form), plus shared services: UI state, notifications, status, support hub, session data, expertise, netquery connection helper, fuzzy search, exit screen, action indicator. Chrome extension project also present (`portmaster-chrome-extension`).

Known UI experiences from code and strings: app list with per-app network activity, connection history with profile/verdict coloring, per-app settings and rules, global settings with expertise-level filtering, overlay map/controls, notifications center, intro/onboarding flow (served as prompt pages), system-theme selection (system/light/dark).

---


---

## 10. Traffic Protection Workflow

1. App opens connection → **interception module** captures packet via **nfqueue** on Linux (or kernel driver/WFP on Windows); eBPF + `/proc` for ownership
2. **Conntrack** lookup + **process attribution** (`gopsutil` reads `/proc`) → binary identified; icon/name resolved; special-process handling (Snap, AppImage, scripts on Linux; Windows Store apps, svchost on Windows)
3. **Profile** matched → per-app rules applied; profiles support fingerprints, endpoints, and rules; profiles can be merged
4. **Filter lists** (`base.dsdl`, DSD/DSDL msgpack, base/intermediate/urgent tiers) → domain/IP verdicts; DNS name resolution verified (CNAME chains, blocked scopes)
5. **DNS**: embedded **nameserver** (`miekg/dns`, default `localhost:53`) resolves/verifies names; intercepts "astray" DNS queries and reroutes to itself; DoT/DoH resolvers supported; split horizon and horizon validation defend against rebinding attacks; `dnsmonitor`/netenv detect captive portals via varlink to NetworkManager + systemd-resolved
6. **GeoIP/country** rules via MaxMind MMDBs
7. **Verdict deciders** (in order): own-connection allowance, broadcast reply, connection type, connection scope, endpoint lists, invalid IP, resolver scope, connectivity domain, bypass prevention, filter lists, custom filter list, domain heuristics, auto-permit related
8. Verdict (allow/block/drop/ask) → connection recorded in **history** (local DB, retention ~14 days, auto-delete or on-demand) → pushed to UI over websocket + notifications; bandwidth tracked per connection/app

Default actions: permit/ask/deny per profile; DNS requests always default-allowed; unknown process/profile → deny. Interception can be disabled via flag (`--disable-interception`, breaks most functionality).

---

## 11. Privacy Overlay Network (SPN)

A privacy network positioned between VPN and Tor: onion-style encryption over multiple hops; routes chosen to maximize distance within the network; exits chosen near the destination (auto geo-unblocking); per-app/domain exclusion; per-app routing algorithm and focus; nodes hosted by the vendor and community; speeds >100 Mbit/s observed claim.

Architecture packages:
- **access** — user accounts, authentication tokens, subscription/feature gating, zones
- **cabin** — identity, keys, verification (public config, identity DB)
- **captain** — client controller: login, hub bootstrap, announcements, gossip, navigation, status, hooks
- **crew** — connections: connect/ping, sticky peers, policy
- **docks** — hub multiplexing: crane (terminal expansion), capacity/latency/sync/expand ops, measurements, metrics, hub import
- **hub** — announcements/status/intel, transport, truststores, updates
- **navigator** — route finding: map, regions, routing profiles, costs, optimization, nearest-exit, measurements
- **patrol** — health checks: domain/http connectivity
- **ships** — transports: TCP, KCP, HTTP, virtual network, masking, pier, MTU, launch
- **sluice** — local listener/tunnel entry: packet listener, UDP listener, request handling
- **terminal** — multiplexed streams/frames/sessions, rate limiting, permissions
- **unit** — scheduling (scheduler stats, debug)

Crypto: `rot256/pblind` (blinding), `aead/ecdh`, `aead/serpent`, Fortuna CSPRNG (`seehuhn/fortuna`). Tunnels established to bootstrap hubs shipped in intel data; overlay state gated by release level (beta/experimental).

---

## 12. System Service & Security Model

- Runs as **root** via systemd with ~14 Linux capabilities (NET_ADMIN, NET_RAW, BPF, SYS_MODULE, SYS_PTRACE, DAC_OVERRIDE, FOwner, …)
- Heavy sandboxing: `MemoryDenyWriteExecute`, `PrivateTmp`, `ProtectSystem=true`, `ProtectHome=read-only`, `RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_INET AF_INET6`, `RestrictNamespaces`, `PrivateDevices`, `LockPersonality`
- `ExecStopPost` flushes iptables rules on service stop (`-recover-iptables`)
- Core binds `127.0.0.1:817` loopback only (configurable `--api-address`)
- UI process is **not** the security boundary: stopping the UI leaves the firewall running
- Windows: kernel extension (WDK driver, `windows_kext/wdk/src`: filter engine, IRP handling, spin locks, RW locks, attributes, debug) + core DLL (`windows_core_dll`); WFP integration; IP Helper API for process attribution

Update security: binary and intel updaters use `jess.TrustStore` for signed update verification; auto-download and auto-apply enabled by default for overlay builds; update resources served with `Resource-Version`/`Resource-SHA256` headers and optional YAML↔JSON content conversion.

DNS security: horizon validation, rebinding defense, split horizon support; captive-portal detection.

---

## 13. Key Dependencies

Core (Go): `gopacket` (packet decode/assembly), `go-nfqueue`/`go-conntrack` (netfilter), `cilium/ebpf` (eBPF), `coreos/go-iptables` (iptables), `miekg/dns` (DNS), `oschwald/maxminddb-golang` (GeoIP), `gorilla/mux` + `gorilla/websocket` (HTTP API + WS), `bbolt` + `modernc.org/sqlite` + `zombiezen.com/go/sqlite` (databases), `godbus/dbus/v5` (D-Bus), `varlink/go` (NetworkManager), `safing/jess` (signed updates/trust store), `VictoriaMetrics/metrics` (metrics), `stephenafamo/bob` + `rubenv/sql-migrate` (migrations), `spf13/cobra` (CLI), `seehuhn/fortuna` (CSPRNG), `ivpn` client protocol (VPN integration).

UI shell (Rust/Tauri): `tauri` 2.2.5, `tauri-plugin-shell` 2.2.1, `tauri-plugin-dialog`, `tauri-plugin-clipboard-manager`, `tauri-plugin-os`, `tauri-plugin-single-instance`, `tauri-plugin-notification`, `tauri-plugin-log`, `tauri-plugin-window-state`, `tauri-plugin-websocket`, `tokio`, `tokio-websockets`, `notify-rust` (Linux), `tauri_winrt_notification` (Windows).

Frontend (Angular 16): `@angular/*`, `ng-zorro-antd`, `@fortawesome/fontawesome`, `d3`, `fuse.js`, `@tauri-apps/api` + plugins, `emoji-toolkit`, `ngx-markdown`.

---

## 14. Component Inventory Summary

| Layer | Component | Technology |
|-------|-----------|------------|
| Daemon | Core service | Go ELF, 45 MB, pure-Go (`CGO_ENABLED=0`) |
| UI shell | Desktop app | Rust/Tauri 2 ELF, 27 MB |
| Web UI | Angular SPA | Angular 16.2.12, ~1,160 bundle files, served by core |
| Assets | Icons/favicons | ZIP archive served at `/assets/` |
| Service | systemd unit | Root, ~14 capabilities, sandboxed |
| Data dir | State + intel | Databases (bbolt/SQLite), logs, intel data |
| Intelligence | Block lists + GeoIP | DSD/DSDL msgpack lists, MaxMind MMDBs, overlay map |
| API | Local HTTP + WS | `127.0.0.1:817/api/v1`, auth + permissions |
| Overlay network | Privacy tunnel | Multi-hop onion-encrypted, Go |

---

## 15. MCP / LLM / AI Integration

**None found.** A repository-wide search for "model context protocol", "mcp", "llm", and "llms.txt" across Go, TypeScript, Rust, Markdown, and JSON files returned no Model Context Protocol server, no LLM/AI agent interface, and no `llms.txt`. The only incidental matches were false positives in unrelated files (a network location module, package-lock metadata, a host metrics file). The developer-facing surface is the local HTTP/WebSocket API described in §6, plus pprof/debug endpoints and a command-line one-shot operation mechanism.

---

## 16. Evidence & Limitations

**Evidence artifacts collected:**
- Installed binary inspection: systemd unit, `/usr/bin`, `/usr/lib/portmaster`, `/var/lib/portmaster/intel`, journalctl logs, version string (`2.2.8`, Go 1.26.10, commit `5c820354…`)
- Source tree: `portmaster-development.zip` (2,443 files, version 2.2.9 dev, 2026-11-11): `service/instance.go`, `service/mgr`, `base/api`, `base/config`, `service/firewall/*`, `service/network/*`, `service/process/*`, `service/profile/*`, `spn/*`, `desktop/angular/*`, `desktop/tauri/*`, `packaging/*`, `windows_kext/*`, `Earthfile`, GitHub workflows
- Exports: `--export-api-endpoints`, `--export-config-options`

**Limitations:**
1. **Static analysis only.** The core service was deliberately not started (it hijacks the host network stack via nfqueue/iptables/eBPF). Runtime behavior — actual packet verdicts, websocket traffic, overlay tunnel establishment, update downloads — is inferred from code and strings, not observed.
2. **API export captured early boot only.** `--export-api-endpoints` runs before module registration, so module-level endpoints (profiles, connections, filter lists, overlay control) are inferred from registration call sites, not from a live registry.
3. **Frontend semantic coverage partial.** Bundle analysis parsed 1,159 files / 1,326 modules but some large chunks have truncated semantic graphs; route list and framework version are solid, fine-grained component behavior is not fully mapped.
4. **Intel data snapshot.** Filter lists/GeoIP on disk reflect the installed update feed; content changes with auto-updates.

---

## 17. Unknowns

- Exact default per-app rules and default filter-list selection (values live in the running DB, not exported)
- Default history retention days (code references `%d days` placeholder; constant not extracted)
- Default API listen address when `--api-address` is unset (strings indicate config-driven; no hardcoded default found)
- Whether CGO remains disabled at runtime (build says `CGO_ENABLED=0`; pure-Go libc present)
- Full overlay topology and encryption handshake details (inferred from package names + strings; not decompiled)
- Exact UI component hierarchy beyond routes (semantic graph truncated in places)
- Auto-update server endpoints and signature verification key material (updater uses `jess.TrustStore`; trust store contents not inspected)
- Whether any telemetry/metrics are pushed externally (Prometheus push-gateway URL is configurable; default behavior not confirmed)
