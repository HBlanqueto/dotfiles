{ pkgs, ... }:

{
  home = {
    packages = [ pkgs.uv ];
    sessionPath = [ "$HOME/.local/bin" ];
  };

  programs.opencode = {
    enable = true;
    package = pkgs.opencode;

    agents = {
      reviewer = ''
        ---
        description: Reviews uncommitted changes for correctness and regressions. Use immediately after writing or modifying code, before any commit.
        mode: subagent
        model: opencode/kimi-k2.7-code
        permissions:
          - action: edit
            resource: "*"
            effect: deny
        ---

        You are a senior code reviewer for a Rust/Wayland codebase (Smithay compositor, Freya-based toolkit).

        1. Run git diff to see the uncommitted changes; focus only on those files.

        Report strictly in this format:

        1. Summary: what you reviewed and overall assessment
        2. Critical Issues: data loss, unsoundness, security — must fix before commit
        3. Major Issues: quality, architecture misalignment, performance
        4. Minor Issues: style, docs, small optimizations
        5. Obstacles Encountered: commands that needed special flags, environment quirks
        6. Verdict: ready to commit or requires changes, with one reason

        Never edit files.
      '';

      second-opinion = ''
        ---
        description: Second opinion on architecture or design decisions that are hard to reverse. Consult before committing to risky refactors or protocol changes.
        mode: subagent
        model: opencode/deepseek-v4.1-flash
        permissions:
          - action: edit
            resource: "*"
            effect: deny
        ---

        You are a design reviewer for a Rust/Wayland codebase (Smithay compositor, Freya-based toolkit). You receive one architecture decision.

        Respond only with:

        1. Overlooked Risks: what the proposal misses
        2. Alternatives: unconsidered options worth a look
        3. Obstacles Encountered: anything unclear or lacking context
        4. Verdict: proceed or reconsider, with one reason

        Never edit files.
      '';
    };

    settings = {
      plugin = [ "@dietrichgebert/ponytail" ];

      mcp = {
        "mcp-nixos" = {
          type = "local";
          command = [ "${pkgs.mcp-nixos}/bin/mcp-nixos" ];
        };
      };
    };
  };
}
