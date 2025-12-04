//! 测试50行输出限制功能

#[cfg(test)]
mod tests {
    use aiw::supervisor::{ScrollingDisplay, DEFAULT_MAX_DISPLAY_LINES};

    #[test]
    fn test_scrolling_display_strict_line_limit() {
        let mut display = ScrollingDisplay::new(3); // 测试限制3行

        // 测试添加行数不超过限制
        let mut output = display.process(b"line 1\n");
        assert_eq!(display.current_line_count(), 1);
        assert!(output.contains("line 1"));

        let mut output = display.process(b"line 2\n");
        assert_eq!(display.current_line_count(), 2);
        assert!(output.contains("line 2"));

        let mut output = display.process(b"line 3\n");
        assert_eq!(display.current_line_count(), 3);
        assert!(output.contains("line 3"));

        // 测试超过限制时自动移除最旧行
        let mut output = display.process(b"line 4\n");
        assert_eq!(display.current_line_count(), 3, "应该严格保持3行限制");
        assert!(display.validate_line_limit(), "应该符合3行限制");
        assert_eq!(display.current_line_count(), 3);
    }

    #[test]
    fn test_scrolling_display_line_processing() {
        let mut display = ScrollingDisplay::new(2);

        // 测试多字节字符处理
        let text = "测试行1\n测试行2\n测试行3\n";
        let output = display.process(text.as_bytes());

        // 验证只有最后2行
        assert_eq!(display.current_line_count(), 2);
        assert!(display.validate_line_limit());
    }

    #[test]
    fn test_scrolling_display_flush_remaining() {
        let mut display = ScrollingDisplay::new(2);

        // 处理不完整的行
        let output = display.process(b"incomplete line without newline");
        assert!(!display.current_line_buffer.is_empty());

        // 刷新剩余内容
        let remaining = display.flush_remaining();
        assert!(remaining.contains("incomplete line without newline"));
        assert!(display.current_line_buffer.is_empty());
    }

    #[test]
    fn test_default_constant() {
        // 验证默认常量确实是50
        assert_eq!(DEFAULT_MAX_DISPLAY_LINES, 50);
    }

    #[test]
    fn test_redraw_function() {
        let mut display = ScrollingDisplay::new(2);
        display.process(b"line 1\nline 2\n");

        let redraw_output = display.redraw();
        assert!(redraw_output.contains("line 1"));
        assert!(redraw_output.contains("line 2"));
        assert_eq!(display.displayed_count, 2);
    }
}