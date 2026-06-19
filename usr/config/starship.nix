{
  format = "$directory$hostname$git_branch$git_status$git_commit$character";

  right_format = ''
    $cmd_duration
  '';

  add_newline = true;

  cmd_duration = {
    min_time = 2000;
    format = "[took](bold bright-yellow) [$duration]($style) ";
    style = "yellow bold";
  };

  directory = {
    format = "[█](fg:black bg:none)[$path]($style)[█ ](fg:black bg:none)";
    style = "fg:bold bright-blue bg:black";
    truncation_length = 3;
    truncate_to_repo = false;
    read_only = " ";
  };

  character = {
    format = "$symbol";
    success_symbol = "[❯](bold bright-red)[❯](bold bright-yellow)[❯ ](bold bright-green)";
    error_symbol = "[❯](bold bright-red)[❯](bold bright-red)[❯ ](bold bright-red)";
  };
}