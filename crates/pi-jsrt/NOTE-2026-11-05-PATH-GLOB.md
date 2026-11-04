2026-11-05

# Node path glob semantics

Owner Codex; EMgine VPM vm-p62g under vm-hbjr. ACC /home/oumy/work/emgine/config/man/anvil.md. Architecture principles 3, 6, 13, 21, 24, 27, 29, 30 and 35 govern reuse, offline dependencies, portable semantics and target evidence. ToQ: both path variants match Node's glob behavior, including Windows separators and host case rules, without a Node or npm runtime; ToT: reuse Node v22.15.1's already bundled minimatch and its original licenses, loaded only on first use.

The existing regex matcher omits brace, extglob and other minimatch semantics. Embed the same self-contained minimatch bundle used by the reference Node release, preserve the ISC/MIT license notices for minimatch, brace-expansion and balanced-match, and invoke it with Node's exact path.matchesGlob options. Keep it separate and lazy through Pi's existing glue mechanism. Verify a fixed cross-platform Node reference corpus, invalid arguments and the existing Pi compatibility suite on Linux; repeat on native Windows and macOS when their runtimes are available. Risk: host-specific case behavior and win32 separators must not be inferred from the build host.

Updated: 2026-11-05. The bundled matcher is loaded through the existing lazy glue interface, with its three upstream license texts alongside the source. On Linux, Pi QuickJS passed 1,020 fixed Node-reference glob cases, 1,147 Windows-path cases and all eight existing Node-compatibility tests. Windows/macOS expectations use the same upstream matcher with Node's host case options, but execution on those hosts remains unverified until the Windows runtime compiles and a macOS run is available.
