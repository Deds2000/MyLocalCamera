package com.localcamera.app;

import com.getcapacitor.BridgeActivity;
import com.localcamera.app.service.AppForeground;

public class MainActivity extends BridgeActivity {
    // F2-T12: o StreamingService só inicia o stream com o app visível
    // (armadilha §6 — câmera/FGS de câmera não pode iniciar do background).
    @Override
    public void onResume() {
        super.onResume();
        AppForeground.onActivityResumed();
    }

    @Override
    public void onPause() {
        AppForeground.onActivityPaused();
        super.onPause();
    }
}
