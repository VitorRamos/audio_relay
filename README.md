### Stream audio from the pc to the phone
Ultra low latency audio relay

### Build server
cd server
cargo b

### Build android lib
cd server
cross b --target=aarch64-linux-android

### Build apk + android lib
./gradlew build