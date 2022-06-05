#include "dbus_utils.h"

using namespace std;

/* Abort on any allocation failure; there is nothing else we can do. */
static void handle_oom(dbus_bool_t success)
{
    if (!success)
    {
        cerr << "Ran out of memory" << endl;
        exit(1);
    }
}

vector<string> get_players(){
    vector<string> res;
    DBusConnection *connection;
    DBusError error;
    DBusMessage *message;
    DBusMessage *reply;
    dbus_bool_t print_reply = TRUE;
    dbus_bool_t print_reply_literal = FALSE;
    int reply_timeout = -1;
    DBusMessageIter iter;
    DBusBusType type = DBUS_BUS_SESSION;
    const char *dest = "org.freedesktop.DBus";
    string name = "org.freedesktop.DBus.ListNames";
    string path = "/org/freedesktop/DBus";

    dbus_error_init(&error);
    connection = dbus_bus_get(type, &error);

    string last_dot = name.substr(name.find_last_of(".")+1);
    name = name.substr(0, name.find_last_of("."));

    message = dbus_message_new_method_call(NULL,
                                            path.data(),
                                            name.data(),
                                            last_dot.data());
    handle_oom(message != NULL);
    dbus_message_set_auto_start(message, TRUE);
 
    if (dest && !dbus_message_set_destination(message, dest))
    {
        cerr << "Not enough memory" << endl;
        return res;
    }

    dbus_message_iter_init_append(message, &iter);

    dbus_error_init(&error);
    reply = dbus_connection_send_with_reply_and_block(connection,
                                                        message, reply_timeout,
                                                        &error);
    if (dbus_error_is_set(&error))
    {
        cerr <<  "Error " << error.name << ": " << error.message << endl;
        return res;
    }

    if (reply)
    {
        DBusMessageIter iter;
        int current_type;
        DBusMessageIter subiter;

        dbus_message_iter_init(reply, &iter);
        dbus_message_iter_recurse(&iter, &subiter);
        current_type = dbus_message_iter_get_arg_type(&subiter);
        while (current_type != DBUS_TYPE_INVALID)
        {
            char *val;
            dbus_message_iter_get_basic(&subiter, &val);
            if(strstr(val,"org.mp") != NULL)
                res.push_back(val);
            dbus_message_iter_next(&subiter);
            current_type = dbus_message_iter_get_arg_type(&subiter);
        }
        dbus_message_unref(reply);
    }

    dbus_message_unref(message);
    dbus_connection_unref(connection);
    return res;
}

void dbus_media_control(string name)
{
    DBusConnection *connection;
    DBusError error;
    DBusMessage *message;
    DBusMessageIter iter;
    DBusBusType type = DBUS_BUS_SESSION;
    int reply_timeout = -1;
    int message_type = DBUS_MESSAGE_TYPE_METHOD_CALL;

    // vector<string> players = get_players();
    // if(players.size() == 0)
    //     return;
    // char *dest = players[0].data(); // fix
    const char *dest = "playerctld"; // fix
    cout << dest << endl;
    char path[] = "/org/mpris/MediaPlayer2";

    dbus_error_init(&error);

    connection = dbus_bus_get(type, &error);

    if (connection == NULL)
    {
        cerr << "Failed to open connection to session message bus: " << error.message << endl;
        dbus_error_free(&error);
        return;
    }
    string last_dot = name.substr(name.find_last_of(".")+1);
    name = name.substr(0, name.find_last_of("."));

    message = dbus_message_new_method_call(NULL,
                                            path,
                                            name.data(),
                                            last_dot.data());
    handle_oom(message != NULL);
    dbus_message_set_auto_start(message, TRUE);

    if (message == NULL)
    {
        cerr << "Couldn't allocate D-Bus message" << endl;
        return;
    }

    if (dest && !dbus_message_set_destination(message, dest))
    {
        cerr << "Not enough memory" << endl;
        return;
    }

    DBusMessage *reply;
    dbus_error_init(&error);
    reply = dbus_connection_send_with_reply_and_block(connection,
                                                        message, reply_timeout,
                                                        &error);
    if (dbus_error_is_set(&error))
    {
        cerr << "Error " << error.name << " " << error.message << endl;        
    }
    if (reply)
    {
        long sec, usec;
        dbus_message_unref(reply);
    }

    dbus_message_unref(message);
    dbus_connection_unref(connection);
}