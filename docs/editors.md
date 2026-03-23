# Editor Integration

ffmt includes a built-in LSP server for real-time format-on-save.

## LSP server

```bash
ffmt --lsp
```

## VS Code

Add to your workspace settings:

```json
{
    "fortran.formatting.formatter": "ffmt",
    "fortran.formatting.args": ["--stdin-filepath", "${file}", "-"]
}
```

## Vim/Neovim

### Format on save

```vim
autocmd BufWritePost *.fpp,*.f90 silent !ffmt %
```

### As formatprg

```vim
set formatprg=ffmt\ -
```

### Neovim LSP

```lua
vim.lsp.start({
    name = "ffmt",
    cmd = { "ffmt", "--lsp" },
    root_dir = vim.fs.dirname(
        vim.fs.find({ "ffmt.toml", "pyproject.toml" }, { upward = true })[1]
    ),
})
```

## Emacs

```elisp
(defun ffmt-buffer ()
  "Format the current buffer with ffmt."
  (interactive)
  (let ((tmpfile (make-temp-file "ffmt" nil ".fpp")))
    (write-region nil nil tmpfile)
    (shell-command (format "ffmt %s" tmpfile))
    (revert-buffer t t t)
    (delete-file tmpfile)))

(add-hook 'f90-mode-hook
          (lambda () (add-hook 'before-save-hook #'ffmt-buffer nil t)))
```
