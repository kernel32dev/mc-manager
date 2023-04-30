@echo -----------------------------------------------------
@echo Esse script tem que ser executado como administrador!
@echo -----------------------------------------------------
curl https://raw.githubusercontent.com/kernel32dev/mc-manager/master/release/mc-manager.exe -o mc-manager.exe
curl https://raw.githubusercontent.com/kernel32dev/mc-manager/master/scripts/mc-update.bat -o mc-update.bat
mc-manager.exe install
mc-manager.exe start
pause