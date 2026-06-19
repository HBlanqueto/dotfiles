{
        #execute_on_song_change = " ";
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
        visualizer_look = "●●";
        visualizer_color = "33,39,63,75,81,99,117,153,189";

        playlist_display_mode = "columns";
        browser_display_mode = "columns";
        user_interface = "classic";

        startup_screen = "visualizer";
        startup_slave_screen = "playlist"; 
        locked_screen_width_part = "33";

        startup_slave_screen_focus = "yes";
        header_window_color = "default";
        alternative_ui_separator_color = "default";
        titles_visibility = "no";

        ask_before_clearing_playlists = "no";
        ask_for_locked_screen_width_part = "yes";

        header_visibility = "no";
        autocenter_mode = "yes";
        centered_cursor = "yes";
        mouse_support = "yes";

        progressbar_look = "━●󰨓";
        progressbar_color = "black";
        progressbar_elapsed_color = "white";

        empty_tag_marker = " ";
        window_border_color = "black";
        active_window_border = "black";
        song_status_format = "$5 { %b }|{ %t }|{ %f }";

        song_columns_list_format = "(53)[white]{tr} (45)[white]{a}";
        song_library_format = "{{%a - %t} (%b)}|{%f}";

        now_playing_prefix = "$b$1$7 ";
        now_playing_suffix = "  $/b$8";
        current_item_prefix = "$b$5$/b$3 ";
        current_item_suffix = "  $4";
}
