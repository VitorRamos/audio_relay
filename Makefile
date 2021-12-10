CXX = g++
LIBS = -lpulse-simple -lpulse -lpthread

server_pulse : server_pulse.cpp
	$(CXX) $? $(LIBS) -o $@ 

clean:
	rm server_pulse