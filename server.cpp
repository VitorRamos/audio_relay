#include <iostream>
#include <alsa/asoundlib.h>

// #include <unistd.h>
// #include <sys/types.h>
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

    uint8_t *buffer;
    int nframes = 512, dir = 0;
    unsigned int rate = 44100;
    snd_pcm_t *handler;
    snd_pcm_hw_params_t *params;
    snd_pcm_format_t fmt = SND_PCM_FORMAT_S16_LE;
    snd_pcm_uframes_t frames, buffer_size = 65536;

    int err = snd_pcm_open(&handler, "default", SND_PCM_STREAM_CAPTURE, 0);
    if(err < 0) return -1;
    cout <<  "audio interface opened" << endl;

    err = snd_pcm_hw_params_malloc(&params);
    if(err < 0) return -1;
    cout << "hw_params allocated" << endl;

    err = snd_pcm_hw_params_any(handler, params);
    if(err < 0) return -1;
    cout << "hw_params initialized" << endl;

    err = snd_pcm_hw_params_set_access(handler, params, SND_PCM_ACCESS_RW_INTERLEAVED);
    if(err < 0) return -1;
    cout << "hw_params access setted" << endl;

    err = snd_pcm_hw_params_set_format(handler, params, fmt);
    if(err < 0) return -1;
    cout << "hw_params format setted" << endl;

    err = snd_pcm_hw_params_set_rate_near(handler, params, &rate, &dir);
    if(err < 0) return -1;
    cout << "hw_params rate setted" << endl;

    err = snd_pcm_hw_params_set_channels(handler, params, 2);
    if(err < 0) return -1;
    cout << "hw_params channels setted" << endl;

    err = snd_pcm_hw_params_set_buffer_size_near(handler, params, &buffer_size);
    if(err < 0) return -1;
    cout << "hw_params buffer size setted" << endl;

    err = snd_pcm_hw_params_get_period_size_min(params, &frames, 0);
    if(err < 0) return -1;
    cout << "Min frame size " << frames << endl;

    err = snd_pcm_hw_params_set_period_size_near(handler, params, &frames, &dir);
    if(err < 0) return -1;
    cout << "hw_params frames size setted" << endl;

    err = snd_pcm_hw_params(handler, params);
    if(err < 0) return -1;
    cout << "hw_params setted" << endl;

    snd_pcm_hw_params_free(params);
    cout << "hw_params freed" << endl;

    err = snd_pcm_prepare(handler);
    if(err < 0) return -1;
    cout << "audio interface prepared" << endl;

    err = snd_pcm_start(handler);
    if(err < 0) return -1;
    cout << "audio interface started" << endl;

    buffer = new uint8_t[nframes*2*2];

    int i = 0;
    while(1) {
        err = snd_pcm_readi(handler, buffer, nframes);
        int n = sendto(sockfd, buffer, nframes*2*2, 0, (sockaddr*)&cliaddr, len);
        cout << "Sending " << n << " bytes " << i++  << endl;
        if(err != nframes) {
            cerr << "read from audio interface failed " << endl;
            return -1;
        }
    }
    delete buffer;
    snd_pcm_close(handler);
    
    return 0;
}