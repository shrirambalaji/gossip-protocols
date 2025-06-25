# PowerShell wrapper script for invoking the Maelstrom jar, with arguments.
# This is the Windows PowerShell equivalent of the bash script.

$ScriptDir = Split-Path -Parent $PSCommandPath
$JarPath = Join-Path $ScriptDir "jar\maelstrom.jar"

# Execute Java with the Maelstrom jar and pass through all arguments
& java "-Djava.awt.headless=true" -jar $JarPath @args
