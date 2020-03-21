if (Test-Path .\target\release\racemus.zip -PathType Leaf) {
    Remove-Item .\target\release\racemus-windows.zip -Force
}
Compress-Archive -Path .\target\release\racemus.exe -DestinationPath .\target\release\racemus-windows.zip
Compress-Archive -Update -Path .\target\release\racemus.pdb -DestinationPath .\target\release\racemus-windows.zip
Compress-Archive -Update -Path .\server.toml -DestinationPath .\target\release\racemus-windows.zip
Compress-Archive -Update -Path .\generate-key.ps1 -DestinationPath .\target\release\racemus-windows.zip
