package com.example.pcstream;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioTrack;
import android.media.MediaMetadata;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.os.Binder;
import android.os.IBinder;
import android.util.Log;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;

import io.reactivex.rxjava3.core.ObservableEmitter;
import io.reactivex.rxjava3.core.Observable;

public class AudioService extends Service {
    private Observable<String> serveip;
    private ObservableEmitter<String> serverip_observer;
    private final IBinder binder = new LocalBinder();
    private DatagramSocket socket_stream = null, socket_cmds = null;
    private boolean running = true;
    private String prev_ip = "";

    private Notification.Builder notification_builder;
    private NotificationManager notification_manager;
    private Thread runner;
    public boolean aptx = true;

    public native int init_decode_rust();
    public native void decode_rust(byte[] input, byte[] output);
    static {
        System.loadLibrary("aptx_rust");
    }

    public class LocalBinder extends Binder {
        AudioService getService() {
            return AudioService.this;
        }
    }

    public Observable<String> get_serverip(){
        return serveip;
    }

    public DatagramSocket create_socket(Integer port) throws Exception {
        DatagramSocket udpSocket = null;
        int MAX_TRIES = 10, cnt = 0;
        while (udpSocket == null && cnt < MAX_TRIES) {
            try {
                udpSocket = new DatagramSocket(null);
                udpSocket.setReuseAddress(true);
                udpSocket.setBroadcast(true);
                udpSocket.setSoTimeout(5000);
                InetSocketAddress address = new InetSocketAddress("0.0.0.0", port);
                udpSocket.bind(address);
            } catch (IOException e) {
                e.printStackTrace();
            }
            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
                e.printStackTrace();
            }
            Log.d("PCstream", "Trying to create socket");
            cnt += 1;
        }
        if (udpSocket == null)
            throw new Exception("Error creating socket");
        Log.d("PCstream", "Socket created successfully");
        return udpSocket;
    }

    @Override
    public void onCreate(){
        int init_dec = 0;
        while(init_dec == 0){
            init_dec = init_decode_rust();
            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
                e.printStackTrace();
            }
            Log.d("PCstream", "Trying to init aptx ctx");
        }
        int chunk = 2048;
        AudioTrack player = new AudioTrack.Builder()
                .setAudioAttributes(new AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build())
                .setAudioFormat(new AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(48000)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                        .build())
                .setBufferSizeInBytes(chunk)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build();
        player.play();

        if(serveip == null) {
            serveip = Observable.create(emitter -> serverip_observer = emitter);
            serveip = serveip.share();
        }

        MediaSession media_session = new MediaSession(getApplicationContext(), getPackageName());
        PlaybackState.Builder state_builder = new PlaybackState.Builder();

        media_session.setActive(true);
        state_builder.setActions( PlaybackState.ACTION_PLAY
                | PlaybackState.ACTION_PLAY_PAUSE | PlaybackState.ACTION_PAUSE
                | PlaybackState.ACTION_SKIP_TO_NEXT | PlaybackState.ACTION_SKIP_TO_PREVIOUS
                );
        state_builder.setState(PlaybackState.STATE_PLAYING, 0, 1.0f);
        media_session.setPlaybackState(state_builder.build());
        media_session.setMetadata(new MediaMetadata.Builder()
                .putText(MediaMetadata.METADATA_KEY_TITLE,"Streaming from")
                .build());
        media_session.setCallback(new MediaSession.Callback() {
            @Override
            public void onPlay() {
                Log.d("PCstream", "PLAY");
            }

            @Override
            public void onPause() {
            }

            @Override
            public void onStop() {
            }

            @Override
            public void onSkipToPrevious() {
                Thread cmd_thread = new Thread(() -> {
                    socket_cmds = null;
                    try {
                        String cmd = "PREV\0";
                        socket_cmds = new DatagramSocket();
                        DatagramPacket sendPacket = new DatagramPacket(cmd.getBytes(StandardCharsets.UTF_8),
                                cmd.length(), InetAddress.getByName(prev_ip.substring(1)), 4053);
                        socket_cmds.send(sendPacket);
                    } catch (IOException e) {
                        e.printStackTrace();
                    }
                });
                cmd_thread.start();
                Log.d("PCstream", "PREV");
            }

            @Override
            public void onSkipToNext() {
                Thread cmd_thread = new Thread(() -> {
                    socket_cmds = null;
                    try {
                        String cmd = "NEXT\0";
                        socket_cmds = new DatagramSocket();
                        DatagramPacket sendPacket = new DatagramPacket(cmd.getBytes(StandardCharsets.UTF_8),
                                cmd.length(), InetAddress.getByName(prev_ip.substring(1)), 4053);
                        socket_cmds.send(sendPacket);
                    } catch (IOException e) {
                        e.printStackTrace();
                    }
                });
                cmd_thread.start();
                Log.d("PCstream", "NEXT");
            }
        });

        String media_channel_id = "pc_stream_playback";
        NotificationChannel media_channel = new NotificationChannel(media_channel_id, "media channel",
                NotificationManager.IMPORTANCE_DEFAULT);
        notification_builder = new Notification.Builder(getApplicationContext(), media_channel_id)
                    .setVisibility(Notification.VISIBILITY_PUBLIC)
                    .setSmallIcon(R.drawable.ic_launcher_foreground)
                    .setStyle(new Notification.MediaStyle()
                            .setShowActionsInCompactView(1)
                            .setMediaSession(media_session.getSessionToken()))
                    .setContentTitle("PCstream")
                    .setContentText("ip")
                    .setOngoing(true)
                    .setShowWhen(false);

        notification_manager = (NotificationManager)getSystemService(Context.NOTIFICATION_SERVICE);
        notification_manager.createNotificationChannel(media_channel);
        notification_manager.notify(0, notification_builder.build());

        try {
            socket_stream = create_socket(4051);
        } catch (Exception e) {
            e.printStackTrace();
        }
        runner = new Thread(() -> {
            byte[] message_aptx = new byte[chunk/4];
            DatagramPacket packet_aptx = new DatagramPacket(message_aptx, chunk/4);
            byte[] message = new byte[chunk];
            DatagramPacket packet_no_aptx = new DatagramPacket(message, chunk);
            DatagramPacket packet;
            int pkg_count = 0;
            while (running) {
                try {
                    if(aptx) {
                        packet = packet_aptx;
                    }
                    else {
                        packet = packet_no_aptx;
                    }
                    socket_stream.receive(packet);
                    if(packet.getAddress().toString() != prev_ip){
                        if(serverip_observer != null)
                            serverip_observer.onNext(prev_ip);
                        prev_ip = packet.getAddress().toString();
                        notification_builder.setContentText(prev_ip);
                        notification_manager.notify(0, notification_builder.build());
                    }
                    if(aptx) {
                        decode_rust(message_aptx, message);
                    }
                    player.write(message, 0, chunk);
                    pkg_count += 1;
                } catch (Exception e) {
                    Log.d("PCstream", "Something bad happen");
                    e.printStackTrace();
                    if(serverip_observer != null)
                        serverip_observer.onNext("disconnected");
                    try {
                        socket_stream.close();
                        socket_stream = create_socket(4051);
                    } catch (Exception ee) {
                        ee.printStackTrace();
                    }
                }
            }
        });
        runner.start();
    }

    @Override
    public void onDestroy()
    {
        running = false;
        try {
            runner.join();
        } catch (InterruptedException e) {
            e.printStackTrace();
        }

        serverip_observer.onNext("disconnected");
        notification_manager.cancel(0);
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return binder;
    }
}