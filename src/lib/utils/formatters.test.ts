import { describe, it, expect } from 'vitest';
import {
  formatBytes,
  formatSpeed,
  formatDuration,
  calculateETA,
  getFileExtension,
  isImageFile,
  isVideoFile,
  getFileIcon,
  extractFileName,
  extractFolderName,
} from './formatters';

describe('formatBytes', () => {
  it('should return "0 B" for 0 bytes', () => {
    expect(formatBytes(0)).toBe('0 B');
  });

  it('should return "0 B" for undefined/NaN', () => {
    expect(formatBytes(NaN)).toBe('0 B');
    expect(formatBytes(undefined as unknown as number)).toBe('0 B');
  });

  it('should format bytes correctly', () => {
    expect(formatBytes(500)).toBe('500.00 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1536)).toBe('1.50 KB');
  });

  it('should format kilobytes correctly', () => {
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB');
    expect(formatBytes(1024 * 1024 * 1.5)).toBe('1.50 MB');
  });

  it('should format megabytes correctly', () => {
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.00 GB');
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toBe('2.50 GB');
  });

  it('should format gigabytes correctly', () => {
    expect(formatBytes(1024 * 1024 * 1024 * 1024)).toBe('1.00 TB');
  });
});

describe('formatSpeed', () => {
  it('should format speed with /s suffix', () => {
    expect(formatSpeed(1024)).toBe('1.00 KB/s');
    expect(formatSpeed(1024 * 1024)).toBe('1.00 MB/s');
  });

  it('should handle 0 speed', () => {
    expect(formatSpeed(0)).toBe('0 B/s');
  });
});

describe('formatDuration', () => {
  it('should format seconds correctly', () => {
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(59)).toBe('59s');
  });

  it('should format minutes correctly', () => {
    expect(formatDuration(60)).toBe('1m 0s');
    expect(formatDuration(90)).toBe('1m 30s');
    expect(formatDuration(150)).toBe('2m 30s');
  });

  it('should format hours correctly', () => {
    expect(formatDuration(3600)).toBe('1h 0m');
    expect(formatDuration(3660)).toBe('1h 1m');
    expect(formatDuration(7200)).toBe('2h 0m');
    expect(formatDuration(7320)).toBe('2h 2m');
  });

  it('should round seconds properly', () => {
    expect(formatDuration(45.7)).toBe('46s');
    expect(formatDuration(45.2)).toBe('45s');
  });
});

describe('calculateETA', () => {
  it('should calculate ETA correctly', () => {
    // 1MB total, 500KB uploaded, 100KB/s = 5 seconds remaining
    expect(calculateETA(1024 * 1024, 512 * 1024, 102400)).toBe(5);
  });

  it('should return 0 when speed is 0', () => {
    expect(calculateETA(1024 * 1024, 0, 0)).toBe(0);
  });

  it('should handle completion', () => {
    expect(calculateETA(1024, 1024, 100)).toBe(0);
  });
});

describe('getFileExtension', () => {
  it('should extract file extension', () => {
    expect(getFileExtension('document.pdf')).toBe('pdf');
    expect(getFileExtension('image.PNG')).toBe('png');
    expect(getFileExtension('archive.tar.gz')).toBe('gz');
  });

  it('should return empty string for no extension', () => {
    expect(getFileExtension('README')).toBe('');
    expect(getFileExtension('Makefile')).toBe('');
  });

  it('should handle edge cases', () => {
    expect(getFileExtension('.gitignore')).toBe('gitignore');
    expect(getFileExtension('')).toBe('');
  });
});

describe('isImageFile', () => {
  it('should identify image files', () => {
    expect(isImageFile('photo.jpg')).toBe(true);
    expect(isImageFile('photo.jpeg')).toBe(true);
    expect(isImageFile('image.png')).toBe(true);
    expect(isImageFile('icon.gif')).toBe(true);
    expect(isImageFile('photo.webp')).toBe(true);
    expect(isImageFile('vector.svg')).toBe(true);
    expect(isImageFile('bitmap.bmp')).toBe(true);
  });

  it('should return false for non-image files', () => {
    expect(isImageFile('document.pdf')).toBe(false);
    expect(isImageFile('video.mp4')).toBe(false);
    expect(isImageFile('README')).toBe(false);
  });

  it('should be case insensitive', () => {
    expect(isImageFile('PHOTO.JPG')).toBe(true);
    expect(isImageFile('Image.PNG')).toBe(true);
  });
});

describe('isVideoFile', () => {
  it('should identify video files', () => {
    expect(isVideoFile('movie.mp4')).toBe(true);
    expect(isVideoFile('clip.webm')).toBe(true);
    expect(isVideoFile('video.mov')).toBe(true);
    expect(isVideoFile('film.avi')).toBe(true);
    expect(isVideoFile('video.mkv')).toBe(true);
    expect(isVideoFile('stream.flv')).toBe(true);
    expect(isVideoFile('movie.wmv')).toBe(true);
  });

  it('should return false for non-video files', () => {
    expect(isVideoFile('document.pdf')).toBe(false);
    expect(isVideoFile('image.png')).toBe(false);
    expect(isVideoFile('song.mp3')).toBe(false);
  });
});

describe('getFileIcon', () => {
  it('should return image icon for images', () => {
    expect(getFileIcon('photo.jpg')).toBe('🖼️');
    expect(getFileIcon('image.png')).toBe('🖼️');
  });

  it('should return video icon for videos', () => {
    expect(getFileIcon('movie.mp4')).toBe('🎥');
    expect(getFileIcon('clip.mov')).toBe('🎥');
  });

  it('should return PDF icon for PDFs', () => {
    expect(getFileIcon('document.pdf')).toBe('📄');
  });

  it('should return document icon for Word files', () => {
    expect(getFileIcon('report.doc')).toBe('📝');
    expect(getFileIcon('report.docx')).toBe('📝');
  });

  it('should return spreadsheet icon for Excel files', () => {
    expect(getFileIcon('data.xls')).toBe('📊');
    expect(getFileIcon('data.xlsx')).toBe('📊');
  });

  it('should return archive icon for compressed files', () => {
    expect(getFileIcon('archive.zip')).toBe('📦');
    expect(getFileIcon('archive.rar')).toBe('📦');
    expect(getFileIcon('archive.tar')).toBe('📦');
    expect(getFileIcon('archive.gz')).toBe('📦');
  });

  it('should return music icon for audio files', () => {
    expect(getFileIcon('song.mp3')).toBe('🎵');
    expect(getFileIcon('audio.wav')).toBe('🎵');
    expect(getFileIcon('music.flac')).toBe('🎵');
  });

  it('should return default document icon for unknown types', () => {
    expect(getFileIcon('unknown.xyz')).toBe('📄');
    expect(getFileIcon('README')).toBe('📄');
  });
});

describe('extractFileName', () => {
  it('should extract file name from Unix paths', () => {
    expect(extractFileName('/Users/test/Documents/file.txt')).toBe('file.txt');
    expect(extractFileName('/home/user/image.png')).toBe('image.png');
    expect(extractFileName('/file.pdf')).toBe('file.pdf');
  });

  it('should extract file name from Windows paths', () => {
    expect(extractFileName('C:\\Users\\test\\Documents\\file.txt')).toBe('file.txt');
    expect(extractFileName('D:\\Projects\\image.png')).toBe('image.png');
    expect(extractFileName('C:\\file.pdf')).toBe('file.pdf');
  });

  it('should handle mixed path separators', () => {
    expect(extractFileName('C:\\Users/test\\Documents/file.txt')).toBe('file.txt');
  });

  it('should return "unknown" for empty paths', () => {
    expect(extractFileName('')).toBe('unknown');
    expect(extractFileName(null as unknown as string)).toBe('unknown');
    expect(extractFileName(undefined as unknown as string)).toBe('unknown');
  });

  it('should return file name for just a file name', () => {
    expect(extractFileName('file.txt')).toBe('file.txt');
  });
});

describe('extractFolderName', () => {
  it('should extract folder name from Unix paths', () => {
    expect(extractFolderName('/Users/test/Documents')).toBe('Documents');
    expect(extractFolderName('/home/user/projects')).toBe('projects');
  });

  it('should extract folder name from Windows paths', () => {
    expect(extractFolderName('C:\\Users\\test\\Documents')).toBe('Documents');
    expect(extractFolderName('D:\\Projects\\MyProject')).toBe('MyProject');
  });

  it('should handle trailing slashes', () => {
    expect(extractFolderName('/Users/test/Documents/')).toBe('Documents');
    expect(extractFolderName('C:\\Users\\test\\Documents\\')).toBe('Documents');
  });

  it('should return "unknown" for empty paths', () => {
    expect(extractFolderName('')).toBe('unknown');
  });
});
