### Stream audio from the pc to the phone
Ultra low latency audio relay

### Build server
cd server && cargo b -r

### Install server service
cargo install --path server
cp pcstream.service /etc/systemd/user/pcstream.service
systemctl enable --user pcstream.service

### Build android lib
cd server
cross b -r --target=aarch64-linux-android

### Build apk + android lib
./gradlew build
