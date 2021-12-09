package com.example.pcstream;

import androidx.appcompat.app.AppCompatActivity;

import android.content.Context;
import android.content.Intent;
import android.net.DhcpInfo;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.os.StrictMode;
import android.util.Log;
import android.widget.Button;
import android.widget.TextView;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;


class SharedData {
    public String server_ip = "";
    public String prev_server_ip = "";
    public boolean update(){
        if(prev_server_ip != server_ip){
            prev_server_ip = server_ip;
            return true;
        }
        return false;
    }
}

class BrodcastAdress implements Runnable {
    public Context context;
    public BrodcastAdress(Context context){
        this.context = context;
    }
    InetAddress getBroadcastAddress() throws IOException {
        WifiManager wifi = (WifiManager) context.getSystemService(Context.WIFI_SERVICE);
        DhcpInfo dhcp = wifi.getDhcpInfo();
        // handle null somehow

        int broadcast = (dhcp.ipAddress & dhcp.netmask) | ~dhcp.netmask;
        byte[] quads = new byte[4];
        for (int k = 0; k < 4; k++)
            quads[k] = (byte) ((broadcast >> k * 8) & 0xFF);
        return InetAddress.getByAddress(quads);
    }
    public void sendBroadcast(String messageStr) {
        // Hack Prevent crash (sending should be done using an async task)
        StrictMode.ThreadPolicy policy = new   StrictMode.ThreadPolicy.Builder().permitAll().build();
        StrictMode.setThreadPolicy(policy);

        try {
            //Open a random port to send the package
            DatagramSocket socket = new DatagramSocket();
            socket.setBroadcast(true);
            byte[] sendData = messageStr.getBytes();
            DatagramPacket sendPacket = new DatagramPacket(sendData, sendData.length, getBroadcastAddress(), 4052);
            socket.send(sendPacket);
            Log.d("PCstream", getClass().getName() + "Broadcast packet sent to: " + getBroadcastAddress().getHostAddress());
        } catch (IOException e) {
            Log.e("PCstream", "IOException: " + e.getMessage());
        }
    }
    @Override
    public void run() {
        sendBroadcast("Iam a server");
    }
}

public class MainActivity extends AppCompatActivity {
    public Intent audio_service;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        SharedData data = new SharedData();
        //AudioClient x = new AudioClient(data);
        //Thread client = new Thread(x);
        audio_service = new Intent(this, AudioService.class);
        //audio_service.putExtra("data", data);
        startService(audio_service);

        new Thread(() -> {
            while (true) {
                try {
                    if(data.update()){
                        runOnUiThread(() -> {
                            TextView sip = findViewById(R.id.server_ip);
                            sip.setText(data.server_ip);
                        });
                    }
                    Thread.sleep(1000);
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
            }
        }).start();
        //startService(new Intent(getBaseContext(), AudioService.class));

        Button bntt_brodcast = findViewById(R.id.bntt_brodcast);
        bntt_brodcast.setOnClickListener(v -> {
            BrodcastAdress badress = new BrodcastAdress(v.getContext());
            new Thread(badress).start();
        });
    }

    public void onDestroy() {
        super.onDestroy();
        stopService(audio_service);
        Log.d("PCStream", "Stopping audio service");
    }
}