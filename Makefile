CXX = g++
LIBS = -lpulse-simple -lpulse -lpthread

server_pulse : server_pulse.cpp
	$(CXX) $? $(LIBS) -o $@ 

install:
	cp server_pulse /usr/bin/server_pulse
	cp pcstream.service /etc/systemd/user/pcstream.service
	systemctl enable --user pcstream.service

clean:
	rm server_pulse