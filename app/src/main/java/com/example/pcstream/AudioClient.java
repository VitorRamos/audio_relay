package com.example.pcstream;

import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioTrack;
import android.util.Log;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetSocketAddress;
import java.util.logging.Handler;

public class AudioClient implements Runnable{

    public SharedData data;

    public AudioClient(SharedData data) {
        this.data = data;
    }

    public DatagramSocket create_socket() throws Exception {
        DatagramSocket udpSocket = null;
        int MAX_TRIES = 10, cnt = 0;
        while (udpSocket == null && cnt < MAX_TRIES) {
            try {
                udpSocket = new DatagramSocket(null);
                udpSocket.setReuseAddress(true);
                udpSocket.setBroadcast(true);
                udpSocket.setSoTimeout(1000);
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
    public void run() {
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
                .setBufferSizeInBytes(2048)
                .build();
        player.play();

        DatagramSocket udpSocket = null;
        try {
            udpSocket = create_socket();
        } catch (Exception e) {
            e.printStackTrace();
        }

        int cnt = 0, numRead;
        byte[] message = new byte[2048];
        while (true) {
            try {
                DatagramPacket packet = new DatagramPacket(message, message.length);
                udpSocket.receive(packet);
                data.server_ip = packet.getAddress().toString();
//                Log.d("PCstream", connected_ip);
                numRead = packet.getLength();
//                Log.d("PCstream", "recebendo");
                player.write(message, 0, numRead);
                cnt += 1;
            } catch (Exception e) {
                Log.d("PCstream", "Something bad happen");
                data.server_ip = "";
                try {
                    udpSocket.close();
                    udpSocket = create_socket();
                } catch (Exception ee) {
                    ee.printStackTrace();
                }
            }
        }
    }
}