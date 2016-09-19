package tickrecorder;

import com.fxcore2.*;
import redis.clients.jedis.*;

public class TickRecorder{
    private static JedisPool redisPool = new JedisPool(Config.redisIP, Config.redisPort);

    public static void main(String[] args){
        O2GSession session = FXCMConnect(Config.FXCMUsername, Config.FXCMPassword, Config.FXCMHostsURL, Config.connectionType);
        
        //printPrices(session);
        try{
            O2GTableManager tableManager = session.getTableManager();
            O2GOffersTable offersTable = (O2GOffersTable)tableManager.getTable(O2GTableType.OFFERS);

            ResponseListener responseListener = new ResponseListener(session);
            session.subscribeResponse(responseListener);

            subscribeToPriceUpdates(offersTable);

            setupRedis(session);
        }catch(java.lang.NullPointerException ex){
            System.out.println("Unable to connect.  Most likely the servers are down for maintenance.  ");
            System.exit(0);
        }
        
        while(true){
            try{
                Thread.sleep(50);
            }catch(InterruptedException e){}
        }
    }
    
    public static O2GSession FXCMConnect(String username, String password, String url, String type){
        O2GSession mSession = O2GTransport.createSession();
        MySessionStatusListener statusListener = new MySessionStatusListener();
        mSession.subscribeSessionStatus(statusListener);
        
        TableManagerListener managerListener = new TableManagerListener();
        mSession.useTableManager(O2GTableManagerMode.YES, managerListener);
        mSession.login(username, password, url, type);
        
        while (!statusListener.isConnected() && !statusListener.hasError()) {
            try{
                Thread.sleep(50);
            }catch(InterruptedException e){}
        }
        
        return mSession;
    }
    
    public static void printPrices(O2GSession loggedInSession){
        O2GTableManager tableManager = loggedInSession.getTableManager();
        O2GOffersTable offersTable = (O2GOffersTable)tableManager.getTable(O2GTableType.OFFERS);
        
        O2GTableIterator iterator = new O2GTableIterator();
        O2GOfferTableRow offerTableRow = offersTable.getNextRow(iterator);
        while (offerTableRow!=null)
        {
            System.out.println("Instrument = " + offerTableRow.getInstrument() + "; Bid = " + offerTableRow.getBid() + "; Ask = " + offerTableRow.getAsk());
            offerTableRow = offersTable.getNextRow(iterator);
        }
    }
    
    private static void subscribeToPriceUpdates(O2GOffersTable offersTable){
        TableListener tableListener = new TableListener();
        offersTable.subscribeUpdate(O2GTableUpdateType.UPDATE, tableListener);
    }
    
    private Jedis getClient(JedisPool pool){
        Jedis jedis = null;
        return pool.getResource();
    }
    
    private static void setupRedis(O2GSession session){
        try(Jedis client = redisPool.getResource()) {
            redisPubsubListener redisListener = new redisPubsubListener(session);
            client.subscribe(redisListener, "priceRequests");
        }catch(redis.clients.jedis.exceptions.JedisConnectionException ex){
            setupRedis(session);
        }
    }
    
    public static void redisPublish(String channel, String message){
        Jedis client = null;
        //System.out.println("Sending redis message on " + channel + ": " + message);
        try{
            client = redisPool.getResource();
            client.publish(channel, message);
        }catch(redis.clients.jedis.exceptions.JedisConnectionException ex){
            //try again.
            redisPublish(channel, message);
        }finally{
            if(client != null){
                client.close();
            }
        }
    }
}
