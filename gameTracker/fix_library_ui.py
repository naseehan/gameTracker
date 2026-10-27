import re
with open('src/pages/Library.tsx', 'r') as f:
    c = f.read()

# Add import
c = c.replace("import { useNavigationStore } from '../store/navigationStore';", "import { useNavigationStore } from '../store/navigationStore';\nimport { Dropdown } from '../components/Dropdown';")

# Filter logic
filter_target = """          <div className="flex items-center space-x-2 bg-gray-900 border border-gray-700 rounded-lg p-1">
            <Filter size={16} className="text-gray-500 ml-2" />
            <select
              value={filterBy}
              onChange={(e) => setFilterBy(e.target.value)}
              className="bg-transparent text-gray-300 text-sm border-none focus:ring-0 cursor-pointer py-1 pr-8"
            >
              <option value="all">All Games</option>
              <option value="installed">Installed</option>
              <option value="not_installed">Not Installed</option>
              <option value="currently_playing">Currently Playing</option>
              <option value="ignored">Ignored</option>
              {platforms.length > 0 && <option disabled>----------</option>}
              {platforms.map(p => (
                <option key={p} value={p}>{p}</option>
              ))}
            </select>
          </div>"""

filter_replacement = """          <Dropdown 
            value={filterBy} 
            onChange={(v) => setFilterBy(v)}
            icon={<Filter size={16} className="text-gray-500" />}
            options={[
              { value: 'all', label: 'All Games' },
              { value: 'installed', label: 'Installed' },
              { value: 'not_installed', label: 'Not Installed' },
              { value: 'currently_playing', label: 'Currently Playing' },
              { value: 'ignored', label: 'Ignored' },
              ...(platforms.length > 0 ? [{ value: 'divider', label: '', disabled: true }] : []),
              ...platforms.map(p => ({ value: p, label: p }))
            ]}
          />"""

c = c.replace(filter_target, filter_replacement)

# Sort logic
sort_target = """          <div className="flex items-center space-x-2 bg-gray-900 border border-gray-700 rounded-lg p-1">
            <SortDesc size={16} className="text-gray-500 ml-2" />
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as SortOption)}
              className="bg-transparent text-gray-300 text-sm border-none focus:ring-0 cursor-pointer py-1 pr-8"
            >
              <option value="recently_played">Recently Played</option>
              <option value="most_played">Most Played</option>
              <option value="recently_added">Recently Added</option>
              <option value="alphabetical">Alphabetical</option>
            </select>
          </div>"""

sort_replacement = """          <Dropdown 
            value={sortBy} 
            onChange={(v) => setSortBy(v as SortOption)}
            icon={<SortDesc size={16} className="text-gray-500" />}
            options={[
              { value: 'recently_played', label: 'Recently Played' },
              { value: 'most_played', label: 'Most Played' },
              { value: 'recently_added', label: 'Recently Added' },
              { value: 'alphabetical', label: 'Alphabetical' }
            ]}
          />"""

c = c.replace(sort_target, sort_replacement)

# Auto Discover button
add_target = """          <button
            onClick={() => setShowAddModal(true)}"""

add_replacement = """          <button
            onClick={() => setShowDiscoveryModal(true)}
            className="flex items-center px-4 py-2 bg-gray-800 hover:bg-gray-700 text-white border border-gray-700 rounded-lg transition-colors font-medium text-sm"
          >
            <Search size={18} className="mr-1" />
            Auto Discover
          </button>
          
          <button
            onClick={() => setShowAddModal(true)}"""

c = c.replace(add_target, add_replacement)


with open('src/pages/Library.tsx', 'w') as f:
    f.write(c)