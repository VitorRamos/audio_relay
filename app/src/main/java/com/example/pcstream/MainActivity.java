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

class BroadcastAddress implements Runnable {
    public Context context;
    public BroadcastAddress(Context context){
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
    private Disposable serverip_disposable;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        serverip_textview = findViewById(R.id.server_ip);
        Button aptx_button = findViewById(R.id.button_aptx);
        Button stop_button = findViewById(R.id.button_stop);
        Button start_button = findViewById(R.id.button_start);

        audio_intent = new Intent(this, AudioService.class);
        startService(audio_intent);
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
        bindService(audio_intent, audio_conn, Context.BIND_AUTO_CREATE);

        aptx_button.setOnClickListener(v -> {
            // TODO change buffer of message size
            audio_service.aptx = !audio_service.aptx;
        });
        stop_button.setOnClickListener(v -> {
            if(audio_conn_bound){
                unbindService(audio_conn);
                audio_conn_bound = false;
            }
            stopService(audio_intent);
            Log.d("PCstream", "Stopping audio service");
        });
        start_button.setOnClickListener(v -> {
            startService(audio_intent);
            if(!audio_conn_bound){
                bindService(audio_intent, audio_conn, 0);
                audio_conn_bound = true;
            }
            Log.d("PCstream", "Starting audio service");
        });

        BroadcastAddress badress = new BroadcastAddress(getApplicationContext());
        new Thread(badress).start();

        Button brodcast_button = findViewById(R.id.button_brodcast);
        brodcast_button.setOnClickListener(v -> {
            BroadcastAddress badress_aux = new BroadcastAddress(v.getContext());
            new Thread(badress_aux).start();
        });
    }

//    @Override
//    public boolean dispatchKeyEvent(KeyEvent event) {
//        Log.d("AAAA", event.toString());
//        if (event.getKeyCode() == KeyEvent.	KEYCODE_HEADSETHOOK) {
//            Toast.makeText(this, "CALLING!", Toast.LENGTH_LONG).show();
//            return true;
//        }
//        return super.dispatchKeyEvent(event);
//    }

    @Override
    public void onResume() {
        super.onResume();
//        if(!audio_conn_bound){
//            bindService(audio_intent, audio_conn, 0);
//            audio_conn_bound = true;
//        }
    }

    @Override
    public void onPause() {
        super.onPause();
//        if(audio_conn_bound) {
//            unbindService(audio_conn);
//            audio_conn_bound = true;
//        }
    }

    @Override
    public void onDestroy() {
        super.onDestroy();
//        if(audio_conn_bound){
//            unbindService(audio_conn);
//            audio_conn_bound = false;
//        }
        stopService(audio_intent);
        Log.d("PCStream", "Stopping audio service");
    }
}