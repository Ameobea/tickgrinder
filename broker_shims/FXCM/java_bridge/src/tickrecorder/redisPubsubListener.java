package tickrecorder;

import redis.clients.jedis.*;
import org.json.simple.*;
import org.json.simple.parser.*;
import com.fxcore2.*;
import java.util.Calendar;

public class redisPubsubListener extends JedisPubSub {
    O2GSession session;
    
    public redisPubsubListener(O2GSession session){
        this.session = session;
    }
        
    public void onMessage(String channel, String message){
        //System.out.println("New redis message recieved on channel " + channel + ": " + message);
        if(channel.equals("priceRequests")){ //JSON format should be this: "[{Pair: "USD/CAD", startTime: 1457300020.23, endTime: 1457300025.57, resolution: t1}]"
            JSONParser parser = new JSONParser();
            JSONArray array = null;
        
            try{
                Object obj = parser.parse(message);
                array = (JSONArray)obj;
            }catch(ParseException e){
                System.out.println("Error parsing JSON message: " + message);
                System.out.println("Error found at " + String.valueOf(e.getPosition()));
            }
            
            JSONObject realParsed = (JSONObject)array.get(0);
            String pair = (String)realParsed.get("pair");
            long startTimeNum = (Long)realParsed.get("startTime");
            long endTimeNum = (Long)realParsed.get("endTime");
            String resolution = (String)realParsed.get("resolution");
            String uuid = (String)realParsed.get("uuid");
            
            Calendar startTime = Calendar.getInstance();
            startTime.setTimeInMillis(startTimeNum);
            Calendar endTime = Calendar.getInstance();
            endTime.setTimeInMillis(endTimeNum);
            HistoryDownloader.downloadHistory(session, pair, resolution, startTime, endTime, uuid);
        }
    }
}
