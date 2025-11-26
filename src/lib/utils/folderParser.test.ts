import { describe, it, expect } from 'vitest';
import {
  parseObjectsIntoFolders,
  getAllFilesInFolder,
  calculateFolderSize,
  getBreadcrumbs,
  type FileNode,
} from './folderParser';
import type { R2Object } from '../types';

describe('parseObjectsIntoFolders', () => {
  it('should parse flat file list', () => {
    const objects: R2Object[] = [
      { key: 'file1.txt', size: 100, lastModified: '2024-01-01', etag: 'abc' },
      { key: 'file2.txt', size: 200, lastModified: '2024-01-02', etag: 'def' },
    ];

    const result = parseObjectsIntoFolders(objects);
    
    expect(result).toHaveLength(2);
    expect(result[0].name).toBe('file1.txt');
    expect(result[0].isFolder).toBe(false);
    expect(result[0].size).toBe(100);
    expect(result[1].name).toBe('file2.txt');
    expect(result[1].size).toBe(200);
  });

  it('should parse nested folder structure', () => {
    const objects: R2Object[] = [
      { key: 'docs/', size: 0, lastModified: '2024-01-01', etag: '' },
      { key: 'docs/readme.md', size: 500, lastModified: '2024-01-01', etag: 'abc' },
      { key: 'docs/api/swagger.json', size: 1000, lastModified: '2024-01-01', etag: 'def' },
    ];

    const result = parseObjectsIntoFolders(objects);
    
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('docs');
    expect(result[0].isFolder).toBe(true);
    expect(result[0].children).toBeDefined();
    expect(result[0].children?.length).toBe(2);
  });

  it('should handle prefix filtering', () => {
    const objects: R2Object[] = [
      { key: 'backup/2024/file.txt', size: 100, lastModified: '2024-01-01', etag: 'abc' },
      { key: 'backup/2024/data.json', size: 200, lastModified: '2024-01-01', etag: 'def' },
    ];

    const result = parseObjectsIntoFolders(objects, 'backup/');
    
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('2024');
    expect(result[0].isFolder).toBe(true);
  });

  it('should handle empty object list', () => {
    const result = parseObjectsIntoFolders([]);
    expect(result).toHaveLength(0);
  });

  it('should handle mixed files and folders', () => {
    const objects: R2Object[] = [
      { key: 'root.txt', size: 50, lastModified: '2024-01-01', etag: 'a' },
      { key: 'folder/', size: 0, lastModified: '2024-01-01', etag: '' },
      { key: 'folder/nested.txt', size: 100, lastModified: '2024-01-01', etag: 'b' },
    ];

    const result = parseObjectsIntoFolders(objects);
    
    expect(result).toHaveLength(2);
    const file = result.find(n => n.name === 'root.txt');
    const folder = result.find(n => n.name === 'folder');
    
    expect(file?.isFolder).toBe(false);
    expect(folder?.isFolder).toBe(true);
    expect(folder?.children?.length).toBe(1);
  });

  it('should preserve file metadata', () => {
    const objects: R2Object[] = [
      { key: 'document.pdf', size: 1024, lastModified: '2024-06-15T10:30:00Z', etag: 'abc123' },
    ];

    const result = parseObjectsIntoFolders(objects);
    
    expect(result[0].size).toBe(1024);
    expect(result[0].lastModified).toBe('2024-06-15T10:30:00Z');
    expect(result[0].etag).toBe('abc123');
  });
});

describe('getAllFilesInFolder', () => {
  it('should return single file for file node', () => {
    const fileNode: FileNode = {
      name: 'file.txt',
      path: 'file.txt',
      isFolder: false,
      size: 100,
      lastModified: '2024-01-01',
    };

    const result = getAllFilesInFolder(fileNode);
    expect(result).toEqual(['file.txt']);
  });

  it('should return all files in folder recursively', () => {
    const folderNode: FileNode = {
      name: 'docs',
      path: 'docs',
      isFolder: true,
      size: 0,
      lastModified: '2024-01-01',
      children: [
        { name: 'readme.md', path: 'docs/readme.md', isFolder: false, size: 100, lastModified: '2024-01-01' },
        { 
          name: 'api',
          path: 'docs/api',
          isFolder: true,
          size: 0,
          lastModified: '2024-01-01',
          children: [
            { name: 'swagger.json', path: 'docs/api/swagger.json', isFolder: false, size: 200, lastModified: '2024-01-01' },
          ]
        },
      ],
    };

    const result = getAllFilesInFolder(folderNode);
    expect(result).toEqual(['docs/readme.md', 'docs/api/swagger.json']);
  });

  it('should return empty array for empty folder', () => {
    const emptyFolder: FileNode = {
      name: 'empty',
      path: 'empty',
      isFolder: true,
      size: 0,
      lastModified: '2024-01-01',
      children: [],
    };

    const result = getAllFilesInFolder(emptyFolder);
    expect(result).toEqual([]);
  });
});

describe('calculateFolderSize', () => {
  it('should return file size for file node', () => {
    const fileNode: FileNode = {
      name: 'file.txt',
      path: 'file.txt',
      isFolder: false,
      size: 1024,
      lastModified: '2024-01-01',
    };

    expect(calculateFolderSize(fileNode)).toBe(1024);
  });

  it('should calculate total size of folder contents', () => {
    const folderNode: FileNode = {
      name: 'docs',
      path: 'docs',
      isFolder: true,
      size: 0,
      lastModified: '2024-01-01',
      children: [
        { name: 'file1.txt', path: 'docs/file1.txt', isFolder: false, size: 100, lastModified: '2024-01-01' },
        { name: 'file2.txt', path: 'docs/file2.txt', isFolder: false, size: 200, lastModified: '2024-01-01' },
      ],
    };

    expect(calculateFolderSize(folderNode)).toBe(300);
  });

  it('should calculate nested folder sizes recursively', () => {
    const folderNode: FileNode = {
      name: 'root',
      path: 'root',
      isFolder: true,
      size: 0,
      lastModified: '2024-01-01',
      children: [
        { name: 'file.txt', path: 'root/file.txt', isFolder: false, size: 100, lastModified: '2024-01-01' },
        {
          name: 'subfolder',
          path: 'root/subfolder',
          isFolder: true,
          size: 0,
          lastModified: '2024-01-01',
          children: [
            { name: 'nested.txt', path: 'root/subfolder/nested.txt', isFolder: false, size: 200, lastModified: '2024-01-01' },
          ],
        },
      ],
    };

    expect(calculateFolderSize(folderNode)).toBe(300);
  });

  it('should return 0 for empty folder', () => {
    const emptyFolder: FileNode = {
      name: 'empty',
      path: 'empty',
      isFolder: true,
      size: 0,
      lastModified: '2024-01-01',
      children: [],
    };

    expect(calculateFolderSize(emptyFolder)).toBe(0);
  });
});

describe('getBreadcrumbs', () => {
  it('should return empty array for empty path', () => {
    expect(getBreadcrumbs('')).toEqual([]);
  });

  it('should return single breadcrumb for root level', () => {
    const result = getBreadcrumbs('documents');
    expect(result).toEqual([{ name: 'documents', path: 'documents' }]);
  });

  it('should return full breadcrumb trail', () => {
    const result = getBreadcrumbs('docs/api/v1/swagger.json');
    expect(result).toEqual([
      { name: 'docs', path: 'docs' },
      { name: 'api', path: 'docs/api' },
      { name: 'v1', path: 'docs/api/v1' },
      { name: 'swagger.json', path: 'docs/api/v1/swagger.json' },
    ]);
  });

  it('should handle trailing slashes', () => {
    const result = getBreadcrumbs('folder/subfolder/');
    expect(result).toEqual([
      { name: 'folder', path: 'folder' },
      { name: 'subfolder', path: 'folder/subfolder' },
    ]);
  });

  it('should handle leading slashes', () => {
    const result = getBreadcrumbs('/folder/file.txt');
    expect(result).toEqual([
      { name: 'folder', path: 'folder' },
      { name: 'file.txt', path: 'folder/file.txt' },
    ]);
  });
});
