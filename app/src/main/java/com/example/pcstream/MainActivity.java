package com.example.pcstream;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.net.DhcpInfo;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.os.IBinder;
import android.os.StrictMode;
import android.util.Log;
import android.widget.Button;
import android.widget.TextView;

import androidx.appcompat.app.AppCompatActivity;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;

import io.reactivex.rxjava3.android.schedulers.AndroidSchedulers;
import io.reactivex.rxjava3.disposables.Disposable;


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

    private Intent audio_intent;
    private AudioService audio_service;
    private ServiceConnection audio_conn;
    private boolean audio_conn_bound = false;
    private TextView serverip_textview;
    private Button brodcast_button;
    private Disposable serverip_disposable;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        serverip_textview = findViewById(R.id.server_ip);
        audio_intent = new Intent(this, AudioService.class);
        audio_conn = new ServiceConnection() {
            @Override
            public void onServiceConnected(ComponentName className, IBinder service) {
                AudioService.LocalBinder binder = (AudioService.LocalBinder) service;
                audio_service = binder.getService();
                audio_conn_bound = true;
                serverip_disposable = audio_service.get_serverip()
                                    .observeOn(AndroidSchedulers.mainThread())
                                    .subscribe(serverip -> serverip_textview.setText(serverip));
            }

            @Override
            public void onServiceDisconnected(ComponentName arg0) {
                audio_conn_bound = false;
            }
        };
        startService(audio_intent);
        bindService(audio_intent, audio_conn, Context.BIND_AUTO_CREATE);

        BrodcastAdress badress = new BrodcastAdress(getApplicationContext());
        new Thread(badress).start();

        brodcast_button = findViewById(R.id.bntt_brodcast);
        brodcast_button.setOnClickListener(v -> {
            BrodcastAdress badress_aux = new BrodcastAdress(v.getContext());
            new Thread(badress_aux).start();
        });
    }

    public void onDestroy() {
        super.onDestroy();
        unbindService(audio_conn);
        audio_conn_bound = false;
        Log.d("PCStream", "Stopping audio service");
    }
}