import random
import argparse
from pathlib import Path

def generate_matrix(output_dir="", rows=5, cols=10):
    """Generate a matrix with random numbers and save it to a file."""
    output_path = Path(output_dir)
    try:
        # Generate the matrix with random integers between 0 and 100
        matrix = [[random.randint(0, 100) for _ in range(cols)] for _ in range(rows)]
        
        path_matrix = output_path/"matrix.txt"
        # Write the matrix to the specified file
        with path_matrix.open('w') as f:
            for row in matrix:
                f.write(' '.join(map(str, row)) + '\n')
        
        print(f"Matrix saved to {path_matrix}")
    except Exception as e:
        print(f"An error occurred: {e}")

def main():
    parser = argparse.ArgumentParser(description='Generate a matrix with random numbers')
    parser.add_argument('--output_dir', default='', help='Output directory')
    parser.add_argument('--rows', type=int, default=5, help='Number of rows in the matrix')
    parser.add_argument('--cols', type=int, default=10, help='Number of columns in the matrix')
    
    args = parser.parse_args()
    generate_matrix(args.output_dir, args.rows, args.cols)

if __name__ == '__main__':
    main()
