# This file allows ncmpcpp or cava to display audio spectrum bars or visualizers.

{}:

{
  audio = {
    output = "tee name=t ! queue ! audioresample ! audioconvert ! audio/x-raw,rate=44100,channels=2,format=S16LE ! pipewiresink t. ! queue ! audioconvert ! audio/x-raw,rate=44100,channels=2,format=S16LE ! udpsink host=127.0.0.1 port=5555";
  };
}