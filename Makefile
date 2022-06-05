CXX = g++
LIBS = -lpulse-simple -lpulse -lpthread -ldbus-1
INCLUDE = -I/usr/include/dbus-1.0 -I/usr/lib/x86_64-linux-gnu/dbus-1.0/include
FILES = server_pulse.cpp pa_utils.cpp dbus_utils.cpp openaptx.c

server_pulse: $(FILES)
	$(CXX) -O3 $(INCLUDE) $(FILES) $(LIBS) -o $@ 

install:
	cp server_pulse /usr/bin/server_pulse
	cp pcstream.service /etc/systemd/user/pcstream.service
	systemctl enable --user pcstream.service

clean:
	rm server_pulse