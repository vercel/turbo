/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export class PackageManagerRoot {
  readonly root: string;
  readonly isSinglePackage: boolean;
  static find(path?: string | undefined | null): Promise<PackageManagerRoot>;
  packageManager(): PackageManager;
  packages(): Promise<Array<Workspace>>;
}
export class PackageManager {
  readonly name: string;
}
export class Workspace {
  readonly absolutePath: string;
  readonly repoPath: string;
}
