// Folder structure utilities for parsing R2 objects into hierarchical tree

import type { R2Object } from '../types';

export interface FileNode {
  name: string;
  path: string;
  isFolder: boolean;
  size: number;
  lastModified: string;
  etag?: string;
  children?: FileNode[];
}

/**
 * Parse flat R2 object list into hierarchical folder structure
 */
export function parseObjectsIntoFolders(objects: R2Object[], currentPrefix: string = ''): FileNode[] {
  const nodes: Map<string, FileNode> = new Map();
  const rootNodes: FileNode[] = [];

  for (const obj of objects) {
    // Remove current prefix to get relative path
    let relativePath = obj.key;
    if (currentPrefix && obj.key.startsWith(currentPrefix)) {
      relativePath = obj.key.substring(currentPrefix.length);
    }

    // Skip if empty after removing prefix
    if (!relativePath) continue;

    const parts = relativePath.split('/').filter(p => p);
    
    // Build folder structure
    let currentPath = currentPrefix;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      currentPath = currentPath ? `${currentPath}/${part}` : part;

      if (!nodes.has(currentPath)) {
        const node: FileNode = {
          name: part,
          path: currentPath,
          isFolder: !isLast || obj.key.endsWith('/'),
          size: isLast ? obj.size : 0,
          lastModified: isLast ? obj.lastModified : new Date().toISOString(),
          etag: isLast ? obj.etag : undefined,
          children: !isLast || obj.key.endsWith('/') ? [] : undefined,
        };
        nodes.set(currentPath, node);

        // Add to parent's children or root
        if (i === 0) {
          rootNodes.push(node);
        } else {
          const parentPath = currentPath.substring(0, currentPath.lastIndexOf('/'));
          const parent = nodes.get(parentPath);
          if (parent && parent.children) {
            parent.children.push(node);
          }
        }
      } else if (isLast && !obj.key.endsWith('/')) {
        // Update file metadata
        const node = nodes.get(currentPath)!;
        node.size = obj.size;
        node.lastModified = obj.lastModified;
        node.etag = obj.etag;
      }
    }
  }

  return rootNodes;
}

/**
 * Get all file paths from a folder node recursively
 */
export function getAllFilesInFolder(node: FileNode): string[] {
  if (!node.isFolder) {
    return [node.path];
  }

  const files: string[] = [];
  if (node.children) {
    for (const child of node.children) {
      files.push(...getAllFilesInFolder(child));
    }
  }
  return files;
}

/**
 * Calculate total size of a folder
 */
export function calculateFolderSize(node: FileNode): number {
  if (!node.isFolder) {
    return node.size;
  }

  let totalSize = 0;
  if (node.children) {
    for (const child of node.children) {
      totalSize += calculateFolderSize(child);
    }
  }
  return totalSize;
}

/**
 * Get breadcrumb path from a file path
 */
export function getBreadcrumbs(path: string): { name: string; path: string }[] {
  if (!path) return [];
  
  const parts = path.split('/').filter(p => p);
  const breadcrumbs: { name: string; path: string }[] = [];
  
  let currentPath = '';
  for (const part of parts) {
    currentPath = currentPath ? `${currentPath}/${part}` : part;
    breadcrumbs.push({ name: part, path: currentPath });
  }
  
  return breadcrumbs;
}
