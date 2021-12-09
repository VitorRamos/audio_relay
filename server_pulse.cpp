#include <iostream>
#include <string.h>

#include <pulse/simple.h>
#include <pulse/error.h>
#include <pulse/stream.h>

#include <arpa/inet.h>
#include <sys/socket.h>
#include <netinet/in.h>

using namespace std;


int main()
{
    int sockfd;
    sockaddr_in cliaddr;
    int len = sizeof(cliaddr);
    memset(&cliaddr, 0, sizeof(cliaddr));

    cliaddr.sin_family = AF_INET;
    cliaddr.sin_addr.s_addr = inet_addr("192.168.0.13");
    cliaddr.sin_port = htons(4051);

    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if(sockfd < 0) return -1;

    pa_simple *s;
    pa_sample_spec ss;
    pa_buffer_attr battr;
    
    ss.format = PA_SAMPLE_S16NE;
    ss.channels = 2;
    ss.rate = 44100;
    
    battr.maxlength = 65536; // max buffer len
    battr.tlength = 2048; // target buffer len
    battr.prebuf = 512; // The server does not start with playback before at least prebuf
    battr.minreq = 512; // The server does not request less than minreq bytes
    battr.fragsize = 2048; // The server sends data in blocks of fragsize bytes size
    
    s = pa_simple_new(NULL,               // Use the default server.
                    "pc relay",           // Our application's name.
                    PA_STREAM_RECORD,
                    "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor",// Use the default device.
                    "System sound",            // Description of our stream.
                    &ss,                // Our sample format.
                    NULL,               // Use default channel map
                    &battr,               // Use default buffering attributes.
                    NULL               // Ignore error code.
                    );
    int error;
    uint8_t buffer[2048];
    while(1){
        if(pa_simple_read(s, buffer, sizeof(buffer), &error) < 0){
            break;
        }
        // pa_simple_flush(s, &error);
        pa_usec_t latency = pa_simple_get_latency(s, &error);
        if(latency > 0)
            cout << latency << endl;
        int n = sendto(sockfd, buffer, sizeof(buffer), 0, (sockaddr*)&cliaddr, len);
    }
}