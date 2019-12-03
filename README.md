# Contrasleuth

[![Join the chat at https://gitter.im/contrasleuth/community](https://badges.gitter.im/contrasleuth/community.svg)](https://gitter.im/contrasleuth/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

**Welcome to Contrasleuth &mdash; a potent communication tool**

For non-native (and non-) speakers of English: the name is pronounced as /ˈkɒntrəsluːθ/. The first syllable is stressed.

Contrasleuth's ambition is to be the largest unmanaged wireless ad hoc network in the world. The network offers two types of service:

- Reliable delivery: You are allowed to send small, expiring pieces of data to one or multiple recipients. When a node receives some data, it stores that piece of data in its inventory. The inventory reconciliation process is run continually to make sure every inventory gets consistent eventually. The network has abuse prevention mechanisms (currently proof of work) to prevent you from flooding the network with lots of data. **We are aware that most software eventually phases out proof of work as an abuse busting measure.**
- Best-effort delivery (akin to the Internet): Packets of data expire in 350ms. When a node receives some data, it immediately forwards the data to its immediate neighbors and only stores the hash of that data until it expires to prevent broadcast radiation. **A node can deny service if it is flooded with traffic.**

In both modes, your data is broadcast to the network. The major differences between both modes are data retention and deliverability guarantees.

The Contrasleuth app functions similarly to an email client. You can send messages to a person or a group with or without the Internet and have them automatically sent to the recipient(s) as quickly as possible. The messages are marshalled in a neat way to provide the best communication experience.

The app is a battery hog but we hope you won't mind. This is still experimental technology. When this piece of tech gets more attention, hopefully the platform will provide some features to improve battery efficiency for this kind of app.
