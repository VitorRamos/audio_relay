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
import android.os.Binder;
import android.os.IBinder;
import android.util.Log;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetSocketAddress;

import io.reactivex.rxjava3.core.ObservableEmitter;
import io.reactivex.rxjava3.core.Observable;

public class AudioService extends Service {
    private Observable<String> serveip;
    private ObservableEmitter<String> serverip_observer;
    private final IBinder binder = new LocalBinder();
    public DatagramSocket udpSocket = null;

//    public AudioService(SharedData data) {
//        this.data = data;
//    }

    public class LocalBinder extends Binder {
        AudioService getService() {
            // Return this instance of LocalService so clients can call public methods
            return AudioService.this;
        }
    }

    public Observable<String> get_serverip(){
        if(serveip == null) {
            serveip = Observable.create(emitter -> serverip_observer = emitter);
            serveip = serveip.share();
        }
        return serveip;
    }

    public DatagramSocket create_socket() throws Exception {
        DatagramSocket udpSocket = null;
        int MAX_TRIES = 10, cnt = 0;
        while (udpSocket == null && cnt < MAX_TRIES) {
            try {
                udpSocket = new DatagramSocket(null);
                udpSocket.setReuseAddress(true);
                udpSocket.setBroadcast(true);
                udpSocket.setSoTimeout(5000);
                InetSocketAddress address = new InetSocketAddress("0.0.0.0", 4051);
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
        int chunk = 2048;
        AudioTrack player = new AudioTrack.Builder()
                .setAudioAttributes(new AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build())
                .setAudioFormat(new AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(44100)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                        .build())
                .setBufferSizeInBytes(chunk)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build();
        player.play();

        MediaSession ms = new MediaSession(getApplicationContext(), getPackageName());
        ms.setActive(true);
        ms.setMetadata(new MediaMetadata.Builder()
                .putText(MediaMetadata.METADATA_KEY_TITLE,"PCStream")
                .build());
        ms.setCallback(new MediaSession.Callback() {
            @Override
            public void onPlay() {
            }

            @Override
            public void onPause() {
            }

            @Override
            public void onStop() {
            }

            @Override
            public void onSkipToPrevious() {
            }

            @Override
            public void onSkipToNext() {
            }
        });
        String channelId = "pc_stream_playback";
        NotificationChannel channel = new NotificationChannel(
                channelId,
                "Channel human readable title",
                NotificationManager.IMPORTANCE_LOW);
        Notification notification = new Notification.Builder(getApplicationContext(), channelId)
                    .setVisibility(Notification.VISIBILITY_PUBLIC)
                    .setSmallIcon(R.drawable.ic_launcher_foreground)
                    .setStyle(new Notification.MediaStyle()
                            .setShowActionsInCompactView(1)
                            .setMediaSession(ms.getSessionToken()))
                    .setContentTitle("Track title")
                    .setContentText("Artist - Album")
                    .build();

//        Notification notification2 = new NotificationCompat.Builder(getApplicationContext(), channelId)
//                // Show controls on lock screen even when user hides sensitive content.
//                .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
//                // Add media control buttons that invoke intents in your media service
////                .addAction(R.drawable.ic_prev, "Previous", prevPendingIntent) // #0
////                .addAction(R.drawable.ic_pause, "Pause", pausePendingIntent)  // #1
////                .addAction(R.drawable.ic_next, "Next", nextPendingIntent)     // #2
//                // Apply the media style template
//                .setSmallIcon(R.drawable.ic_launcher_foreground)
//                .setStyle(new androidx.media.app.NotificationCompat.MediaStyle())
//                .setContentTitle("PCStream")
//                .setContentText("active stream")
//                //.setLargeIcon(albumArtBitmap)
//                .build();
        NotificationManager mNotificationManager = (NotificationManager)getSystemService(Context.NOTIFICATION_SERVICE);
        mNotificationManager.createNotificationChannel(channel);
        mNotificationManager.notify(6, notification);

        try {
            udpSocket = create_socket();
        } catch (Exception e) {
            e.printStackTrace();
        }
        new Thread(new Runnable() {
            @Override
            public void run() {
                DatagramPacket packet = null;
                byte[] message = new byte[chunk];
                packet = new DatagramPacket(message, message.length);
                int cnt = 0, sum = 0, numRead;
                while (true) {
                    try {
                        udpSocket.receive(packet);
                        serverip_observer.onNext(packet.getAddress().toString());
//                Log.d("PCstream", connected_ip);
                        numRead = packet.getLength();
//                Log.d("PCstream", "recebendo");
                        sum = 0;
                        for (byte b : message) sum |= b;
                        if(sum != 0)
                            player.write(message, 0, numRead);
                        cnt += 1;
                    } catch (Exception e) {
                        Log.d("PCstream", "Something bad happen");
                        e.printStackTrace();
                        //data.server_ip = "";
                        try {
                            udpSocket.close();
                            udpSocket = create_socket();
                        } catch (Exception ee) {
                            ee.printStackTrace();
                        }
                    }
                }
            }
        }).start();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return binder;
    }
}