# Hacking on Contrasleuth

**This document is intended for prospective hackers of Contrasleuth. If you are an ordinary user, download the Contrasleuth app from the Play Store.**

## Building this thing

You should have Vagrant installed. Every dependency will be automatically installed the moment you run `vagrant up`. The `build-scripts/` folder contains everything you need to build the project.

In the `build-scripts/` folder, type `yarn run release` in the Vagrant SSH shell. The `release.ts` script runs the necessary tools such as `capnp`, `rust-embedded/cross`, `parcel` and the Android SDK to build the final APK file.

## The innards of Contrasleuth

Contrasleuth is not a native Android app. It is a web application masquerading as an Android app and uses native APIs to facilitates ad hoc network connections. Similar to a normal web app, Contrasleuth also has a _frontend_ which is powered by web technologies and a _backend_, which is implemented in Rust. The frontend doesn't communicate to the backend using HTTP but rather, it invokes the functions exposed by the Android shell to execute interprocess communication operations.

The frontend uses IndexedDB to store persistent data such as cryptographic keys and saved messages. It is secured by a strong Content Security Policy to prevent XSS attacks and is isolated from the rest of the app as the frontend has to process highly sensitive data. Extracting secret information such as cryptographic keys and messages should be impossible unless the IPC handling code is implemented incorrectly. To prevent programming errors, the TypeScript programming language is used to enforce strong type invariants.

The backend is a Rust application cross compiled by `rust-embedded/cross` for Android targets. Before the `cross` tool is invoked, the Cap'n Proto schema compiler is run to generate `backend/artifacts/reconcile.capnp.rs`. It handles low-level operations such as making network inventories consistent and pruning expired messages. It also provides an interface for the frontend to retrieve the raw messages and decrypt them with the frontend keys.

The shell is a simple Android application that wraps over the frontend and the backend. It exposes a JavaScript interface to the frontend for interprocess communication with the backend and connects to nearby devices using Wi-Fi Direct.
