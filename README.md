[![Rust](https://github.com/hschink/fritzer/actions/workflows/rust.yml/badge.svg)](https://github.com/hschink/fritzer/actions/workflows/rust.yml)

# fritzer
A command-line tool for the [AVM Home Automation HTTP Interface](https://avm.de/fileadmin/user_upload/Global/Service/Schnittstellen/AHA-HTTP-Interface.pdf)

# Usage

```bash
# Check out the library (use your preferred approach)
cargo run -- -u http://fritz.box switch -l # lists all switches connected to your Fritz!Box
```

Please consider the following behavior:
1. fritzer uses the last user logged in to the Fritz!Box.
2. After a successful login, fritzer stores the session id (SID) in `~\fritzer.sid`.
3. If fritzer finds `~\fritzer.sid`, fritzer checks if the SID stored in the file is still valid and uses the valid SID before starting a login attempt.

# Alternatives

* [Fritz!Box Tools](https://www.mengelke.de/Projekte/FritzBox-Tools)