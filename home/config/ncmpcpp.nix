{
  ncmpcpp_directory = "~/.config/ncmpcpp";
  lyrics_directory = "~/.config/ncmpcpp/lyrics";
  mpd_host = "127.0.0.1";
  mpd_port = 6600; 
  mpd_connection_timeout = 60;

  visualizer_data_source = "127.0.0.1:5555";
  visualizer_output_name = "Mopidy UDP";
  visualizer_in_stereo = "yes";
  visualizer_type = "ellipse"; 
  visualizer_fps = "30";
  visualizer_look = "󱓻󱓻";
  visualizer_color = "8,4,12,6,14,13,7,15";

  playlist_display_mode = "columns";
  browser_display_mode = "columns";
  user_interface = "classic";

  startup_screen = "playlist";
  startup_slave_screen = "visualizer"; 
  locked_screen_width_part = "50";
  state_line_color = "red";

  startup_slave_screen_focus = "no";
  header_window_color = "red";
  alternative_ui_separator_color = "red";
  titles_visibility = "yes";

  ask_before_clearing_playlists = "no";
  ask_for_locked_screen_width_part = "yes";

  header_visibility = "no";
  autocenter_mode = "yes";
  centered_cursor = "yes";
  mouse_support = "yes";

  progressbar_look = "▂▂▂";
  progressbar_color = "black";
  progressbar_elapsed_color = "white";

  empty_tag_marker = " ";
  song_status_format = "$7 { %b }|{ %t }|{ %f }";

  song_columns_list_format = "(45)[white]{t} (45)[white]{a} (10)[white]{l}";
  song_library_format = "{{%a - %t} (%b)}|{%f}";
}
