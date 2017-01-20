package tickrecorder;

import com.fxcore2.*;
import static tickrecorder.TickRecorder.redisPublish;
import java.util.Date;
import java.text.DecimalFormat;

public class TableListener implements IO2GTableListener {
    public void onAdded(String string, O2GRow o2grow){
        
    }
    
    public void onChanged(String rowID, O2GRow rowData){
        O2GOfferTableRow offerTableRow = (O2GOfferTableRow)(rowData);
        if (offerTableRow!=null){
            for(int i=0;i<Config.monitoredPairs.length;i++){
                if(offerTableRow.getInstrument().equals(Config.monitoredPairs[i])){
                    String json = "{\"real\": true, \"pair\": \"";
                    json += offerTableRow.getInstrument().replace("/", "").toLowerCase();
                    json += "\", \"timestamp\": ";
                    Date now = new Date();
                    DecimalFormat df = new DecimalFormat("#");
                    df.setMaximumFractionDigits(4);
                    json += df.format((double)now.getTime() / 1000);
                    json += ", \"bid\": ";
                    json += String.valueOf(offerTableRow.getBid());
                    json += ", \"ask\": ";
                    json += String.valueOf(offerTableRow.getAsk());
                    json += "}";
                    redisPublish("ticks", json);
                }
            }
        }
    }
    
    public void onDeleted(String string, O2GRow o2grow){
        
    }
    
    public void onStatusChanged(O2GTableStatus ogts){
    
    }
}