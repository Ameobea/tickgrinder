***SETTING THIS SHIT UP***

So there is a lot of work that goes into setting up this particular piece of software.  First of all, you need to add the FXCM trading API as a library.  
The guide for this can be found here: http://fxcodebase.com/bin/forexconnect/1.4.1/help/Java/order2go2_Java.html and I've downloaded a copy in case of disaster here: https://ameo.link/u/1oe.png

The FXCM Jar needs to be in the lib directory, and all the DLLs need to be in the system PATH.  I'm not sure at this point how to set that up for Linux, but for Windows I found it easiest to just
put them into a folder that's already in the PATH, such as system32.  That makes the step about adding the -D parameter for the JVM unnecessary.  

I read somewhere on the website that openJDK or something like that for linux is very unadvised and to use Oracle Java instead.  

For Redis, I used the Jedis library: https://github.com/xetorthio/jedis/releases.  I just added it as a library.

You'll need to include a JSON library as well; I used json-simple: https://code.google.com/archive/p/json-simple/downloads.  Require the jar as a library.