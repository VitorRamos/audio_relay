#include <iostream>
#include <thread>

#include <arpa/inet.h>
#include <sys/socket.h>
#include <netinet/in.h>

#include "dbus_utils.h"
#include "pa_utils.h"

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

void handle_cmds()
{
    int sockfd;
    sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    socklen_t len = sizeof(addr);

    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = inet_addr("0.0.0.0");
    addr.sin_port = htons(4053);

    sockaddr_in caddr;
    
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    bind(sockfd, (sockaddr*)&addr, sizeof(addr));

    char buff[12];
    while(1){
        recvfrom(sockfd, buff, 12, 0, (sockaddr*)&caddr, &len);
        buff[11] = '\0';
        char str[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &(caddr.sin_addr), str, INET_ADDRSTRLEN);
        cout << buff << " " << str << endl;
        if(strcmp(buff, "NEXT")){
            dbus_media_control("org.mpris.MediaPlayer2.Player.Previous");
        }
        if(strcmp(buff, "PREV")){
            dbus_media_control("org.mpris.MediaPlayer2.Player.Next");
        }
    }
}

#include "openaptx.h"

int main(int argc, char** argv)
{
    bool with_aptx = false;
    if(argc == 2){
        if(strcmp(argv[1], "--aptx") == 0){
            with_aptx = true;
        }
    }
    thread reciver(recv_server_addr);
    thread cmds(handle_cmds);
    int sockfd;
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);

    pa_simple *s;
    pa_sample_spec ss;
    pa_buffer_attr battr;
    
    ss.format = PA_SAMPLE_S16LE;
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
    uint8_t output_buffer[2048/4];
    uint8_t decoded_buffer[2048];

    int hd = 0;
    int ret;
    size_t length;
    size_t processed;
    size_t written;
    size_t dropped;
    int synced;
    int syncing;

    struct aptx_context *ctx_enc;
    struct aptx_context *ctx_dec;
    ctx_enc = aptx_init(hd);
    if (!ctx_enc) {
        fprintf(stderr, "Cannot initialize aptX encoder\n");
        return 1;
    }
    ctx_dec = aptx_init(hd);
    if (!ctx_dec) {
        fprintf(stderr, "Cannot initialize aptX encoder\n");
        return 1;
    }

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
        processed = aptx_encode(ctx_enc, buffer, sizeof(buffer), output_buffer, sizeof(output_buffer), &written);
        if (processed != sizeof(buffer))
            break;
        // processed = aptx_decode_sync(ctx_dec, output_buffer, sizeof(output_buffer), decoded_buffer, sizeof(decoded_buffer), &written, &synced, &dropped);
        // /* Check all possible states of synced, syncing and dropped status */
        // if (!synced) {
        //     if (!syncing) {
        //         fprintf(stderr, "aptX decoding failed, synchronizing\n");
        //         syncing = 1;
        //         ret = 1;
        //     }
        //     if (dropped) {
        //         fprintf(stderr, "aptX synchronization successful, dropped %lu byte%s\n", (unsigned long)dropped, (dropped != 1) ? "s" : "");
        //         syncing = 0;
        //         ret = 1;
        //     }
        //     if (!syncing) {
        //         fprintf(stderr, "aptX decoding failed, synchronizing\n");
        //         syncing = 1;
        //         ret = 1;
        //     }
        // } else {
        //     if (dropped) {
        //         if (!syncing)
        //             fprintf(stderr, "aptX decoding failed, synchronizing\n");
        //         fprintf(stderr, "aptX synchronization successful, dropped %lu byte%s\n", (unsigned long)dropped, (dropped != 1) ? "s" : "");
        //         syncing = 0;
        //         ret = 1;
        //     } else if (syncing) {
        //         fprintf(stderr, "aptX synchronization successful\n");
        //         syncing = 0;
        //         ret = 1;
        //     }
        // }
        // /* If we have not decoded all supplied samples then decoding unrecoverable failed */
        // if (processed != sizeof(output_buffer)) {
        //     fprintf(stderr, "aptX decoding failed\n");
        //     ret = 1;
        //     break;
        // }
        // written -= 2*2;
        int n = sendto(sockfd, output_buffer, sizeof(output_buffer), 0, (sockaddr*)&cliaddr, sizeof(cliaddr));
    }
    reciver.join();
    cmds.join();
}