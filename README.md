### Stream audio from the pc to the phone
Ultra low latency audio relay

### Build server
```bash
cd server && cargo b -r
```

### Install service
```bash
cargo install --path server
cp pcstream.service /etc/systemd/user/pcstream.service
systemctl enable --user pcstream.service
```

### Build android lib
```bash
cd server
cross b -r --target=aarch64-linux-android
```

### Build apk + android lib
```bash
./gradlew build
```
