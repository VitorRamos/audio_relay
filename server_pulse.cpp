#include <iostream>
#include <string.h>
#include <thread>

#include <pulse/simple.h>
#include <pulse/error.h>
#include <pulse/stream.h>
#include <pulse/pulseaudio.h>

#include <arpa/inet.h>
#include <sys/socket.h>
#include <netinet/in.h>

using namespace std;

sockaddr_in cliaddr;

string recv_server_addr()
{
    int sockfd;
    sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    socklen_t len = sizeof(addr);

    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = inet_addr("0.0.0.0");
    addr.sin_port = htons(4052);

    cliaddr.sin_family = AF_INET;
    cliaddr.sin_addr.s_addr = inet_addr("192.168.0.13"); // defaut
    cliaddr.sin_port = htons(4051);
    
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    bind(sockfd, (sockaddr*)&addr, sizeof(addr));

    char buff[12];
    while(1){
        recvfrom(sockfd, buff, 12, 0, (sockaddr*)&cliaddr, &len);
        cliaddr.sin_port = htons(4051);
        buff[11] = '\0';
        char str[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &(cliaddr.sin_addr), str, INET_ADDRSTRLEN);
        cout << buff << " " << str << endl;
    }
}

void state_cb(pa_context* context, void* raw) {
    switch(pa_context_get_state(context))
    {
        case PA_CONTEXT_READY:
            *((int*)raw) = 1;
            break;
        case PA_CONTEXT_FAILED:
            *((int*)raw) = -1;
            break;
        case PA_CONTEXT_UNCONNECTED:
        case PA_CONTEXT_AUTHORIZING:
        case PA_CONTEXT_SETTING_NAME:
        case PA_CONTEXT_CONNECTING:
        case PA_CONTEXT_TERMINATED:
            break;
    }
}

void source_list_cb(pa_context* c, const pa_source_info *i, int eol, void *raw) {
    if (eol != 0) {
        return;
    }
    if(string(i->description).find("Monitor of Built-in") != string::npos){
        *((string*)raw) = string(i->name);
        // cout << i->index << " " << i->description << " " << i->name << endl;
    }
}

string get_monitor_name(){
    pa_mainloop* mainloop;
    pa_mainloop_api* mainloop_api;
    pa_context* context;
    int retval, state = 0;
    string name;

    mainloop = pa_mainloop_new();
    mainloop_api = pa_mainloop_get_api(mainloop);
    context = pa_context_new(mainloop_api, "test");

    pa_context_set_state_callback(context, &state_cb, &state);
    if (pa_context_connect(context, NULL, PA_CONTEXT_NOFLAGS, NULL) < 0)
        return "";
    while (state <= 0) {
        if (pa_mainloop_iterate(mainloop, 1, &retval) < 0)
            return "";
    }
    if (state == -1)
        return "";

    pa_operation* op = pa_context_get_source_info_list(context, &source_list_cb, &name);
    while (pa_operation_get_state(op) == PA_OPERATION_RUNNING) {
        pa_mainloop_iterate(mainloop, 1, &retval);
    }

    return name;
}

int main()
{
    thread reciver(recv_server_addr);
    int sockfd;
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);

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
    
    string name = get_monitor_name();
    if(name == ""){
        name = "";
    }
    s = pa_simple_new(NULL,               // Use the default server.
                    "pc relay",           // Our application's name.
                    PA_STREAM_RECORD,
                    name.c_str(),// Use the default device.
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
            cout << "Latency " << latency << endl;
        uint8_t sum = 0;
        // for(int i=0; i<2048; i++) sum |= buffer[i];
        // if(sum != 0)
        int n = sendto(sockfd, buffer, sizeof(buffer), 0, (sockaddr*)&cliaddr, sizeof(cliaddr));
    }
    reciver.join();
}