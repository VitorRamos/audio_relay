#include "pa_utils.h"

using namespace std;

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