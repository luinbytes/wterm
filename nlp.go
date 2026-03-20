package main

import (
	"fmt"
	"regexp"
	"runtime"
	"strings"
)

// CommandPattern represents an NLP pattern and its command generator
type CommandPattern struct {
	Pattern     *regexp.Regexp
	Description string
	Safe        bool
	Category    string
	Generator   func(*regexp.Regexp, string) string
}

// NLPParser handles natural language to command translation
type NLPParser struct {
	patterns []CommandPattern
}

// NewNLPParser creates a parser with all patterns registered
func NewNLPParser() *NLPParser {
	p := &NLPParser{
		patterns: make([]CommandPattern, 0),
	}
	p.setupPatterns()
	return p
}

// addPattern registers a new pattern
func (p *NLPParser) addPattern(pattern string, generator func(*regexp.Regexp, string) string, description string, safe bool, category string) {
	p.patterns = append(p.patterns, CommandPattern{
		Pattern:     regexp.MustCompile("(?i)" + pattern),
		Generator:   generator,
		Description: description,
		Safe:        safe,
		Category:    category,
	})
}

// setupPatterns initializes all NLP patterns
func (p *NLPParser) setupPatterns() {
	// Navigation patterns
	p.addPattern(`go to (.+)`, p.cmdGoTo, "Change directory", true, "navigation")
	p.addPattern(`go back`, p.cmdGoBack, "Go to parent directory", true, "navigation")
	p.addPattern(`show (current directory|current path)`, p.cmdPwd, "Show current directory", true, "navigation")
	p.addPattern(`where am i`, p.cmdPwd, "Show current directory", true, "navigation")

	// File operation patterns
	p.addPattern(`list files (?:sorted|sort) by (size|name|date)`, p.cmdListSorted, "List files sorted", true, "files")
	p.addPattern(`list files`, p.cmdList, "List files", true, "files")
	p.addPattern(`create (folder|directory) (.+)`, p.cmdMkdir, "Create directory", true, "files")
	p.addPattern(`create (?:new )?file (.+)`, p.cmdTouch, "Create empty file", true, "files")
	p.addPattern(`delete (?:the )?file (.+)`, p.cmdDeleteFile, "Delete file", false, "files")
	p.addPattern(`delete (?:the )?(folder|directory) (.+)`, p.cmdDeleteDir, "Delete directory", false, "files")
	p.addPattern(`copy (.+) (?:to|into) (.+)`, p.cmdCopy, "Copy file", true, "files")
	p.addPattern(`move (.+) (?:to|into) (.+)`, p.cmdMove, "Move file", false, "files")

	// System patterns
	p.addPattern(`open (.+)`, p.cmdOpen, "Open file/application", true, "system")
	p.addPattern(`clear|clean`, p.cmdClear, "Clear screen", true, "system")
	p.addPattern(`show disk space`, p.cmdDiskSpace, "Show disk space", true, "system")
	p.addPattern(`show ip address`, p.cmdShowIP, "Show IP address", true, "system")
	p.addPattern(`show my ip`, p.cmdShowIP, "Show IP address", true, "system")
	p.addPattern(`show date`, p.cmdShowDate, "Show date", true, "system")
	p.addPattern(`show time`, p.cmdShowTime, "Show time", true, "system")

	// Search patterns
	p.addPattern(`find files (?:named|containing|with) (.+)`, p.cmdFindFiles, "Find files", true, "search")
	p.addPattern(`find text (.+) (?:in|within) files`, p.cmdFindText, "Find text in files", true, "search")

	// Process patterns
	p.addPattern(`show (?:running )?process(?:es)?`, p.cmdShowProcesses, "Show running processes", true, "process")
	p.addPattern(`kill (?:process )?(.+)`, p.cmdKillProcess, "Kill process", false, "process")

	// Environment patterns
	p.addPattern(`set (?:variable )?(.+) (?:to|equal|=) (.+)`, p.cmdSetVar, "Set environment variable", false, "environment")
	p.addPattern(`show variable (.+)`, p.cmdShowVar, "Show environment variable", true, "environment")
	p.addPattern(`^env$`, p.cmdPrintEnv, "Show environment variables", true, "environment")
	p.addPattern(`show environment(?: variables)?$`, p.cmdPrintEnv, "Show environment variables", true, "environment")
	p.addPattern(`list environment variables`, p.cmdPrintEnv, "List environment variables", true, "environment")
	p.addPattern(`print environment`, p.cmdPrintEnv, "Print environment variables", true, "environment")
	p.addPattern(`display environment`, p.cmdPrintEnv, "Display environment variables", true, "environment")
	p.addPattern(`show env$`, p.cmdPrintEnv, "Show environment variables", true, "environment")
	p.addPattern(`export (.+)=(.+)`, p.cmdExportVar, "Export environment variable", false, "environment")
	p.addPattern(`set env (.+)=(.+)`, p.cmdExportVar, "Set environment variable", false, "environment")
	p.addPattern(`show PATH$`, p.cmdShowPath, "Show PATH variable", true, "environment")
	p.addPattern(`show HOME$`, p.cmdShowHome, "Show HOME variable", true, "environment")
	p.addPattern(`what is \$(.+)`, p.cmdWhatIsVar, "Show value of variable", true, "environment")

	// Git patterns
	p.addPattern(`git init`, p.cmdGitInit, "Initialize git repository", true, "git")
	p.addPattern(`git status`, p.cmdGitStatus, "Show git status", true, "git")
	p.addPattern(`git log`, p.cmdGitLog, "Show commit history", true, "git")
	p.addPattern(`git (?:clone|download) (.+)`, p.cmdGitClone, "Clone repository", true, "git")
	p.addPattern(`git add (?:all|.)`, p.cmdGitAddAll, "Stage all changes", true, "git")
	p.addPattern(`git add (.+)`, p.cmdGitAdd, "Stage file", true, "git")
	p.addPattern(`git commit`, p.cmdGitCommit, "Commit changes", true, "git")
	p.addPattern(`git push`, p.cmdGitPush, "Push to remote", false, "git")
	p.addPattern(`git pull`, p.cmdGitPull, "Pull from remote", false, "git")
	p.addPattern(`git branch`, p.cmdGitBranch, "Show branches", true, "git")
	p.addPattern(`git checkout (.+)`, p.cmdGitCheckout, "Switch branch", true, "git")

	// Network patterns
	p.addPattern(`ping (.+)`, p.cmdPing, "Ping host", true, "network")
	p.addPattern(`trace route to (.+)`, p.cmdTraceRoute, "Trace route to host", true, "network")

	// Download patterns
	p.addPattern(`download (.+)`, p.cmdCurl, "Download file via curl", true, "download")
	p.addPattern(`wget (.+)`, p.cmdWget, "Download file via wget", true, "download")
	p.addPattern(`download (?:from|url) (.+)`, p.cmdCurl, "Download file via curl", true, "download")

	// File property patterns
	p.addPattern(`show hidden files`, p.cmdShowHidden, "Show hidden files", true, "properties")
	p.addPattern(`show (?:file )?(?:attributes|props|properties) (.+)`, p.cmdShowProps, "Show file properties", true, "properties")
	p.addPattern(`hide (?:file )?(.+)`, p.cmdHideFile, "Hide file", false, "properties")

	// Text file patterns
	p.addPattern(`(?:show|read|display|cat) file (.+)`, p.cmdShowFile, "Display file contents", true, "text")
	p.addPattern(`(?:edit|open) (?:file )?(.+)`, p.cmdEditFile, "Edit file", true, "text")

	// Unix alias patterns
	p.addPattern(`^ls$`, p.cmdList, "List files (alias)", true, "alias")
	p.addPattern(`^pwd$`, p.cmdPwd, "Show current directory (alias)", true, "alias")
	p.addPattern(`^mkdir (.+)$`, p.cmdMkdir, "Create directory (alias)", true, "alias")
	p.addPattern(`^touch (.+)$`, p.cmdTouch, "Create empty file (alias)", true, "alias")
	p.addPattern(`^rm (.+)$`, p.cmdRm, "Remove file (alias)", false, "alias")
	p.addPattern(`^gci$`, p.cmdList, "List files (PowerShell alias)", true, "alias")
	p.addPattern(`^gl$`, p.cmdPwd, "Show current directory (PowerShell alias)", true, "alias")

	// Archive patterns
	p.addPattern(`zip (.+) into (.+)`, p.cmdZip, "Create zip archive", true, "archive")
	p.addPattern(`zip (.+) to (.+)`, p.cmdZip, "Create zip archive", true, "archive")
	p.addPattern(`unzip (.+)`, p.cmdUnzip, "Extract zip archive", true, "archive")
	p.addPattern(`extract (.+)`, p.cmdUnzip, "Extract zip archive", true, "archive")
	p.addPattern(`tar (.+)`, p.cmdTar, "Create tar archive", true, "archive")
	p.addPattern(`untar (.+)`, p.cmdUntar, "Extract tar archive", true, "archive")
	p.addPattern(`compress (.+)`, p.cmdGzip, "Compress file with gzip", true, "archive")
	p.addPattern(`decompress (.+)`, p.cmdGunzip, "Decompress gzip file", true, "archive")
	p.addPattern(`gzip (.+)`, p.cmdGzip, "Compress file with gzip", true, "archive")
	p.addPattern(`gunzip (.+)`, p.cmdGunzip, "Decompress gzip file", true, "archive")

	// Docker patterns
	p.addPattern(`docker ps`, p.cmdDockerPs, "List running Docker containers", true, "docker")
	p.addPattern(`docker containers`, p.cmdDockerPs, "List Docker containers", true, "docker")
	p.addPattern(`show containers`, p.cmdDockerPs, "List Docker containers", true, "docker")
	p.addPattern(`docker images`, p.cmdDockerImages, "List Docker images", true, "docker")
	p.addPattern(`docker run (.+)`, p.cmdDockerRun, "Run Docker container", true, "docker")
	p.addPattern(`docker build`, p.cmdDockerBuild, "Build Docker image", true, "docker")
	p.addPattern(`docker stop (.+)`, p.cmdDockerStop, "Stop Docker container", false, "docker")
	p.addPattern(`docker logs (.+)`, p.cmdDockerLogs, "Show Docker container logs", true, "docker")
	p.addPattern(`docker exec (.+) (.+)`, p.cmdDockerExec, "Execute command in Docker container", true, "docker")
	p.addPattern(`docker-compose up`, p.cmdDockerComposeUp, "Start services with docker-compose", true, "docker")
	p.addPattern(`docker-compose down`, p.cmdDockerComposeDown, "Stop services with docker-compose", true, "docker")

	// Package manager patterns
	p.addPattern(`apt install (.+)`, p.cmdAptInstall, "Install package with apt", false, "package")
	p.addPattern(`brew install (.+)`, p.cmdBrewInstall, "Install package with brew", false, "package")
	p.addPattern(`npm install (.+)`, p.cmdNpmInstall, "Install package with npm", false, "package")
	p.addPattern(`pip install (.+)`, p.cmdPipInstall, "Install package with pip", false, "package")
	p.addPattern(`apt search (.+)`, p.cmdAptSearch, "Search package with apt", true, "package")
	p.addPattern(`brew search (.+)`, p.cmdBrewSearch, "Search package with brew", true, "package")
	p.addPattern(`apt update`, p.cmdAptUpdate, "Update package lists with apt", false, "package")
	p.addPattern(`brew update`, p.cmdBrewUpdate, "Update brew", false, "package")
	p.addPattern(`brew upgrade`, p.cmdBrewUpgrade, "Upgrade packages with brew", false, "package")
}

// Parse attempts to translate natural language to a shell command
func (p *NLPParser) Parse(input string) (command string, matched bool, description string) {
	input = strings.TrimSpace(input)
	if input == "" {
		return "", false, ""
	}

	for _, pattern := range p.patterns {
		if pattern.Pattern.MatchString(input) {
			cmd := pattern.Generator(pattern.Pattern, input)
			if cmd != "" {
				return cmd, true, pattern.Description
			}
		}
	}

	// No match - return original input
	return input, false, ""
}

// === Pattern Generators ===

func (p *NLPParser) cmdGoTo(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		target := strings.TrimSpace(matches[1])
		return fmt.Sprintf("cd \"%s\"", target)
	}
	return ""
}

func (p *NLPParser) cmdGoBack(re *regexp.Regexp, input string) string {
	return "cd .."
}

func (p *NLPParser) cmdPwd(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "cd"
	}
	return "pwd"
}

func (p *NLPParser) cmdList(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "dir /b"
	}
	return "ls -la"
}

func (p *NLPParser) cmdListSorted(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) < 2 {
		return p.cmdList(re, input)
	}
	sortBy := strings.ToLower(matches[1])

	if runtime.GOOS == "windows" {
		switch sortBy {
		case "size":
			return "dir /b /o-s"
		case "name":
			return "dir /b /on"
		case "date":
			return "dir /b /o-d"
		}
	}
	// Unix
	switch sortBy {
	case "size":
		return "ls -laS"
	case "name":
		return "ls -la"
	case "date":
		return "ls -lat"
	}
	return "ls -la"
}

func (p *NLPParser) cmdMkdir(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		name := strings.TrimSpace(matches[2])
		return fmt.Sprintf("mkdir \"%s\"", name)
	} else if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		return fmt.Sprintf("mkdir \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdTouch(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		name := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("type nul > \"%s\"", name)
		}
		return fmt.Sprintf("touch \"%s\"", name)
	} else if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("type nul > \"%s\"", name)
		}
		return fmt.Sprintf("touch \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdDeleteFile(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("del \"%s\"", name)
		}
		return fmt.Sprintf("rm \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdDeleteDir(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		name := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("rmdir /s /q \"%s\"", name)
		}
		return fmt.Sprintf("rm -rf \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdCopy(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		src := strings.TrimSpace(matches[1])
		dst := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("copy \"%s\" \"%s\"", src, dst)
		}
		return fmt.Sprintf("cp \"%s\" \"%s\"", src, dst)
	}
	return ""
}

func (p *NLPParser) cmdMove(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		src := strings.TrimSpace(matches[1])
		dst := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("move \"%s\" \"%s\"", src, dst)
		}
		return fmt.Sprintf("mv \"%s\" \"%s\"", src, dst)
	}
	return ""
}

func (p *NLPParser) cmdOpen(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		target := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("start \"%s\"", target)
		}
		if runtime.GOOS == "darwin" {
			return fmt.Sprintf("open \"%s\"", target)
		}
		return fmt.Sprintf("xdg-open \"%s\"", target)
	}
	return ""
}

func (p *NLPParser) cmdClear(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "cls"
	}
	return "clear"
}

func (p *NLPParser) cmdDiskSpace(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "wmic logicaldisk get size,freespace,caption"
	}
	return "df -h"
}

func (p *NLPParser) cmdShowIP(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "ipconfig"
	}
	return "ip addr show"
}

func (p *NLPParser) cmdShowDate(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "echo %date%"
	}
	return "date"
}

func (p *NLPParser) cmdShowTime(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "echo %time%"
	}
	return "date +'%T'"
}

func (p *NLPParser) cmdFindFiles(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("dir /s /b \"%s\"", name)
		}
		return fmt.Sprintf("find . -name \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdFindText(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		text := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("findstr /s \"%s\" *.*", text)
		}
		return fmt.Sprintf("grep -r \"%s\" .", text)
	}
	return ""
}

func (p *NLPParser) cmdShowProcesses(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "tasklist"
	}
	return "ps aux"
}

func (p *NLPParser) cmdKillProcess(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		proc := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("taskkill /PID %s /F", proc)
		}
		return fmt.Sprintf("kill -9 %s", proc)
	}
	return ""
}

func (p *NLPParser) cmdSetVar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		name := strings.TrimSpace(matches[1])
		value := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("set %s=%s", name, value)
		}
		return fmt.Sprintf("export %s=\"%s\"", name, value)
	}
	return ""
}

func (p *NLPParser) cmdShowVar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("echo %%%s%%", name)
		}
		return fmt.Sprintf("echo $%s", name)
	}
	return ""
}

func (p *NLPParser) cmdPrintEnv(re *regexp.Regexp, input string) string {
	return "printenv"
}

func (p *NLPParser) cmdExportVar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		name := strings.TrimSpace(matches[1])
		value := strings.TrimSpace(matches[2])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("set %s=%s", name, value)
		}
		return fmt.Sprintf("export %s=\"%s\"", name, value)
	}
	return ""
}

func (p *NLPParser) cmdShowPath(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "echo %PATH%"
	}
	return "echo $PATH"
}

func (p *NLPParser) cmdShowHome(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "echo %USERPROFILE%"
	}
	return "echo $HOME"
}

func (p *NLPParser) cmdWhatIsVar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("echo %%%s%%", name)
		}
		return fmt.Sprintf("echo $%s", name)
	}
	return ""
}

func (p *NLPParser) cmdGitInit(re *regexp.Regexp, input string) string {
	return "git init"
}

func (p *NLPParser) cmdGitStatus(re *regexp.Regexp, input string) string {
	return "git status"
}

func (p *NLPParser) cmdGitLog(re *regexp.Regexp, input string) string {
	return "git log --oneline -10"
}

func (p *NLPParser) cmdGitClone(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		url := strings.TrimSpace(matches[1])
		return fmt.Sprintf("git clone %s", url)
	}
	return ""
}

func (p *NLPParser) cmdGitAddAll(re *regexp.Regexp, input string) string {
	return "git add ."
}

func (p *NLPParser) cmdGitAdd(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		file := strings.TrimSpace(matches[1])
		return fmt.Sprintf("git add %s", file)
	}
	return ""
}

func (p *NLPParser) cmdGitCommit(re *regexp.Regexp, input string) string {
	return "git commit"
}

func (p *NLPParser) cmdGitPush(re *regexp.Regexp, input string) string {
	return "git push"
}

func (p *NLPParser) cmdGitPull(re *regexp.Regexp, input string) string {
	return "git pull"
}

func (p *NLPParser) cmdGitBranch(re *regexp.Regexp, input string) string {
	return "git branch"
}

func (p *NLPParser) cmdGitCheckout(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		branch := strings.TrimSpace(matches[1])
		return fmt.Sprintf("git checkout %s", branch)
	}
	return ""
}

func (p *NLPParser) cmdPing(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		host := strings.TrimSpace(matches[1])
		return fmt.Sprintf("ping %s", host)
	}
	return ""
}

func (p *NLPParser) cmdTraceRoute(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		host := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("tracert %s", host)
		}
		return fmt.Sprintf("traceroute %s", host)
	}
	return ""
}

func (p *NLPParser) cmdCurl(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		url := strings.TrimSpace(matches[1])
		return fmt.Sprintf("curl -O %s", url)
	}
	return ""
}

func (p *NLPParser) cmdWget(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		url := strings.TrimSpace(matches[1])
		return fmt.Sprintf("wget %s", url)
	}
	return ""
}

func (p *NLPParser) cmdShowHidden(re *regexp.Regexp, input string) string {
	if runtime.GOOS == "windows" {
		return "dir /ah"
	}
	return "ls -la"
}

func (p *NLPParser) cmdShowProps(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("attrib \"%s\"", name)
		}
		return fmt.Sprintf("stat \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdHideFile(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("attrib +h \"%s\"", name)
		}
		return fmt.Sprintf("chmod a-rwx \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdShowFile(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("type \"%s\"", name)
		}
		return fmt.Sprintf("cat \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdEditFile(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("notepad \"%s\"", name)
		}
		return fmt.Sprintf("${EDITOR:-nano} \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdRm(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		name := strings.TrimSpace(matches[1])
		if runtime.GOOS == "windows" {
			return fmt.Sprintf("del \"%s\"", name)
		}
		return fmt.Sprintf("rm \"%s\"", name)
	}
	return ""
}

func (p *NLPParser) cmdZip(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		files := strings.TrimSpace(matches[1])
		archive := strings.TrimSpace(matches[2])
		// zip command works on all platforms
		return fmt.Sprintf("zip -r \"%s\" %s", archive, files)
	}
	return ""
}

func (p *NLPParser) cmdUnzip(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		archive := strings.TrimSpace(matches[1])
		// unzip command works on all platforms
		return fmt.Sprintf("unzip \"%s\"", archive)
	}
	return ""
}

func (p *NLPParser) cmdTar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		files := strings.TrimSpace(matches[1])
		// tar command works on all platforms
		return fmt.Sprintf("tar -czf archive.tar.gz %s", files)
	}
	return ""
}

func (p *NLPParser) cmdUntar(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		archive := strings.TrimSpace(matches[1])
		// tar command works on all platforms
		return fmt.Sprintf("tar -xzf \"%s\"", archive)
	}
	return ""
}

func (p *NLPParser) cmdGzip(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		file := strings.TrimSpace(matches[1])
		// gzip command works on all platforms
		return fmt.Sprintf("gzip \"%s\"", file)
	}
	return ""
}

func (p *NLPParser) cmdGunzip(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		file := strings.TrimSpace(matches[1])
		// gunzip command works on all platforms
		return fmt.Sprintf("gunzip \"%s\"", file)
	}
	return ""
}

func (p *NLPParser) cmdDockerPs(re *regexp.Regexp, input string) string {
	return "docker ps"
}

func (p *NLPParser) cmdDockerImages(re *regexp.Regexp, input string) string {
	return "docker images"
}

func (p *NLPParser) cmdDockerRun(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		image := strings.TrimSpace(matches[1])
		return fmt.Sprintf("docker run %s", image)
	}
	return ""
}

func (p *NLPParser) cmdDockerBuild(re *regexp.Regexp, input string) string {
	return "docker build -t myimage ."
}

func (p *NLPParser) cmdDockerStop(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		container := strings.TrimSpace(matches[1])
		return fmt.Sprintf("docker stop %s", container)
	}
	return ""
}

func (p *NLPParser) cmdDockerLogs(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		container := strings.TrimSpace(matches[1])
		return fmt.Sprintf("docker logs %s", container)
	}
	return ""
}

func (p *NLPParser) cmdDockerExec(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 2 {
		container := strings.TrimSpace(matches[1])
		command := strings.TrimSpace(matches[2])
		return fmt.Sprintf("docker exec %s %s", container, command)
	}
	return ""
}

func (p *NLPParser) cmdDockerComposeUp(re *regexp.Regexp, input string) string {
	return "docker-compose up -d"
}

func (p *NLPParser) cmdDockerComposeDown(re *regexp.Regexp, input string) string {
	return "docker-compose down"
}

func (p *NLPParser) cmdAptInstall(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("sudo apt install -y %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdBrewInstall(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("brew install %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdNpmInstall(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("npm install %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdPipInstall(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("pip install %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdAptSearch(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("apt search %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdBrewSearch(re *regexp.Regexp, input string) string {
	matches := re.FindStringSubmatch(input)
	if len(matches) > 1 {
		pkg := strings.TrimSpace(matches[1])
		return fmt.Sprintf("brew search %s", pkg)
	}
	return ""
}

func (p *NLPParser) cmdAptUpdate(re *regexp.Regexp, input string) string {
	return "sudo apt update"
}

func (p *NLPParser) cmdBrewUpdate(re *regexp.Regexp, input string) string {
	return "brew update"
}

func (p *NLPParser) cmdBrewUpgrade(re *regexp.Regexp, input string) string {
	return "brew upgrade"
}
