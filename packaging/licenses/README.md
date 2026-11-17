# License verification key

`nwp-public.hex` is the Ed25519 **public** key used by the offline license
verifier. It is 32 raw bytes encoded as 64 lowercase hex characters.

The matching **private** key belongs to release/payment infrastructure and must
never be committed here or embedded in a shipped binary. This repository only
contains the public half; the daemon can verify licenses but cannot mint them.

Tests generate ephemeral keypairs at runtime and do not use this key.
