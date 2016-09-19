package tickrecorder;

import com.fxcore2.*;

public class MySessionStatusListener implements IO2GSessionStatus {
    private boolean isConnected = false;
    private boolean hasError = false;
    
    public void onSessionStatusChanged(O2GSessionStatusCode ogssc){
        System.out.println("Session Status Changed with code " + ogssc.toString());
        if(ogssc.toString() == "CONNECTED"){
            isConnected = true;
        }
    }
    
    public void onLoginFailed(String string){
        System.out.println("Login failed!");
        hasError = true;
    }
    
    public boolean isConnected(){
        if(isConnected){
            return true;
        }else{
            return false;
        }
    }
    
    public boolean hasError(){
        if(hasError){
            return true;
        }else{
            return false;
        }
    }
}
