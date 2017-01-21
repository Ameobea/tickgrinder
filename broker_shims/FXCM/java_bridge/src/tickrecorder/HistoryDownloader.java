package tickrecorder;

import com.fxcore2.*;
import java.util.Calendar;

public class HistoryDownloader {
    public static void downloadHistory(O2GSession session, String pair, String resolution, Calendar startTime, Calendar endTime, String uuid){
        O2GRequestFactory factory = session.getRequestFactory();
        try{
            O2GTimeframeCollection timeFrames = factory.getTimeFrameCollection();
            O2GTimeframe timeFrame = timeFrames.get(resolution);
            O2GRequest marketDataRequest = factory.createMarketDataSnapshotRequestInstrument(pair, timeFrame, 300);
            factory.fillMarketDataSnapshotRequestTime(marketDataRequest, startTime, endTime, true);
            TickRecorder.redisPublish("historicalPrices", "{\"type\": \"segmentID\", \"id\": \"" +
                    marketDataRequest.getRequestId() + "\", \"uuid\": \"" + uuid + "\"}");
            session.sendRequest(marketDataRequest);
        }catch(java.lang.NullPointerException ex){
            downloadHistory(session, pair, resolution, startTime, endTime, uuid);
        }
    }
}
