import re
with open('src/pages/Library.tsx', 'r') as f:
    c = f.read()

# Fix AddGameModal
add_target = """      {showAddModal && (
        <AddGameModal 
          onClose={() => setShowAddModal(false)}
          onSuccess={() => {
            setShowAddModal(false);
            fetchGames();
          }}
          onOpenScan={() => setShowScanModal(true)}
          onOpenDiscovery={() => setShowDiscoveryModal(true)}
        />
      )}"""
add_replacement = """      {showAddModal && (
        <AddGameModal 
          onClose={() => setShowAddModal(false)}
        />
      )}"""
c = c.replace(add_target, add_replacement)

# Fix ScanModal
scan_target = """      {showScanModal && (
        <ScanModal
          onClose={() => setShowScanModal(false)}
          onSuccess={() => {
            setShowScanModal(false);
            fetchGames();
          }}
        />
      )}"""
scan_replacement = """      {showScanModal && (
        <ScanModal
          onClose={() => setShowScanModal(false)}
        />
      )}"""
c = c.replace(scan_target, scan_replacement)

# Fix DiscoveryModal
discovery_target = """      {showDiscoveryModal && (
        <DiscoveryModal
          onClose={() => setShowDiscoveryModal(false)}
          onSuccess={() => {
            setShowDiscoveryModal(false);
            fetchGames();
          }}
        />
      )}"""
discovery_replacement = """      {showDiscoveryModal && (
        <DiscoveryModal
          onClose={() => setShowDiscoveryModal(false)}
        />
      )}"""
c = c.replace(discovery_target, discovery_replacement)

with open('src/pages/Library.tsx', 'w') as f:
    f.write(c)