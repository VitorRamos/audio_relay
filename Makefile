CXX = g++
LIBS = -lpulse-simple -lpulse

server_pulse : server_pulse.cpp
	$(CXX) $? $(LIBS) -o $@ 

clean:
	rm server_pulse