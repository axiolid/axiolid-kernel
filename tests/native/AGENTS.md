# Native consumer tests

`cmake-consumer/` exercises only the public C API through the stable `Axiolid::axiolid` target. `test-native-cmake.py` copies it outside the workspace before configuring source, installed, or extracted-archive modes; the consumer must execute both semantic success and typed-refusal paths. Its mutation mode removes a required symbol, header, and package config. `test_native_packaging.py` owns archive/path/binary-identity mutation tests. Keep fixtures platform-neutral; platform-specific setup belongs in `.github/workflows/native.yml`.
