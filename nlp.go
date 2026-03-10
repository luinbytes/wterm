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

	// Network patterns
	p.addPattern(`ping (.+)`, p.cmdPing, "Ping host", true, "network")
	p.addPattern(`trace route to (.+)`, p.cmdTraceRoute, "Trace route to host", true, "network")

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
	p.addPattern(`^rm (.+)$`, p.cmdRm, "Remove file (alias)", false, "alias")
	p.addPattern(`^gci$`, p.cmdList, "List files (PowerShell alias)", true, "alias")
	p.addPattern(`^gl$`, p.cmdPwd, "Show current directory (PowerShell alias)", true, "alias")
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
