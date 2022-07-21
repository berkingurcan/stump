import { Series } from './Series';
import { Tag } from './Tag';

export interface Library {
	/**
	 * The id of the library.
	 */
	id: string;
	/**
	 * The name of the library.
	 */
	name: string;
	/**
	 * The path of the library on disk.
	 */
	path: string;
	/**
	 * The (optional) description of the library.
	 */
	description?: string;
	/**
	 * The date in which the library was last updated. This is usually after a scan. ex: "2022-04-20 04:20:69"
	 */
	updatedAt: Date;
	/**
	 * The series in this library. Will be undefined only if the relation is not loaded.
	 */
	series?: Series[];
	/**
	 * The user assigned tags for the library. ex: ["Comics", "Family"]. Will be undefined only if the relation is not loaded.
	 */
	tags?: Tag[];
}

export interface LibrariesStats {
	bookCount: number;
	seriesCount: number;
	totalBytes: number;
}

export interface CreateLibraryInput extends Omit<Library, 'id' | 'updatedAt' | 'series'> {
	scan: boolean;
}

export interface EditLibraryInput extends Library {
	removedTags?: Tag[];
	scan: boolean;
}
