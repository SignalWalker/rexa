# Rexa

<p align="center">
  <a href="https://crates.io/crates/rexa"><img src="https://img.shields.io/crates/v/rexa" alt="crates.io"/></a>
  <a href="https://github.com/SignalWalker/rexa/commits/main"><img src="https://img.shields.io/github/commits-since/SignalWalker/rexa/0.1.0" alt="commits since last release"/></a>
  <a href="https://docs.rs/rexa"><img src="https://img.shields.io/docsrs/rexa" alt="docs.rs"/></a>
  <a href="https://opensource.org/licenses/lgpl-license"><img src="https://img.shields.io/crates/l/rexa" alt="LGPL 3.0 or later"/></a>
</p>

A library implementing [OCapN](https://github.com/ocapn/ocapn), an object-capabilities protocol simplifying development of peer-to-peer applications.

Not yet fit for actual use; wait until [1.0.0](https://github.com/SignalWalker/rexa/issues/1).

## Motivation

- Decentralized services give more power to users, and tend to be longer-lived than their centralized counterparts (ex. IRC vs. AIM, Skype, etc.)
- It is difficult to build such services, because one must reinvent many wheels to ensure security & privacy for their users

## Usage

- [Examples](./samples)

## See Also

- [1.0.0 Checklist](https://github.com/SignalWalker/rexa/issues/1)

## Etymology

- "R" as in "Rust"
- "Exa" as in:
  - "[exo](https://en.wiktionary.org/wiki/exo-)", a prefix meaning "outer" or "external"
  - "[Exapunks](https://www.zachtronics.com/exapunks/)", a puzzle game about distributed programming

## References

\[1\] Christine Lemmer-Webber and Randy Farmer. 2022. The Heart of Spritely: Distributed Objects and Capability Security. Spritely Networked Communities Institute. Retrieved December 5, 2022 from https://spritely.institute/static/papers/spritely-core.html
\[2\] Christine Lemmer-Webber, Randy Farmer, and Jessica Tallon. 2022. Spritely for Secure Applications and Communities. Spritely Networked Communities Institute. Retrieved December 5, 2022 from https://spritely.institute/static/papers/spritely-for-users.html
